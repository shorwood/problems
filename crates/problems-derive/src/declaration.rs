use heck::{ToKebabCase, ToTitleCase};
use proc_macro2::{Span, TokenTree};
use std::collections::BTreeSet;
use syn::{Attribute, Expr, Ident, Lit, LitStr, ext::IdentExt};

pub(crate) struct Declaration {
    pub(crate) type_uri: LitStr,
    pub(crate) status: Option<u16>,
    pub(crate) title: LitStr,
    pub(crate) detail: Option<LitStr>,
}

const STATUS_MESSAGE: &str = "status requires an integer literal from 100 through 999, such as status = 409; omit status to use 500";

fn status_number(expression: Expr) -> syn::Result<u16> {
    let message = STATUS_MESSAGE;
    let Expr::Lit(literal) = expression else {
        return Err(syn::Error::new_spanned(expression, message));
    };
    let Lit::Int(integer) = literal.lit else {
        return Err(syn::Error::new_spanned(literal, message));
    };
    let number = integer
        .base10_parse::<u16>()
        .map_err(|_| syn::Error::new(integer.span(), message))?;
    if !(100..=999).contains(&number) {
        return Err(syn::Error::new(integer.span(), message));
    }
    Ok(number)
}

pub(crate) fn prefix(attributes: &[Attribute]) -> syn::Result<Option<LitStr>> {
    let mut prefix = None;
    for attribute in attributes.iter().filter(|a| a.path().is_ident("problem")) {
        attribute.parse_nested_meta(|meta| {
            if !meta.path.is_ident("prefix") {
                return Err(meta.error("expected prefix on the enum; put declarations on variants"));
            }
            if prefix.is_some() {
                return Err(meta.error("duplicate problem prefix"));
            }
            let value = meta.value()?.parse::<LitStr>()?;
            if value.value().is_empty() {
                return Err(syn::Error::new(
                    value.span(),
                    "problem prefix must not be empty",
                ));
            }
            prefix = Some(value);
            Ok(())
        })?;
    }
    Ok(prefix)
}

pub(crate) fn declaration(
    attributes: &[Attribute],
    span: Span,
    variant: &Ident,
    prefix: Option<&LitStr>,
) -> syn::Result<Declaration> {
    let (mut type_uri, mut status, mut title, mut detail) = (None, None, None, None);
    let mut seen = BTreeSet::new();
    for attribute in attributes.iter().filter(|a| a.path().is_ident("problem")) {
        let first = attribute
            .meta
            .require_list()?
            .tokens
            .clone()
            .into_iter()
            .next();
        if first.is_some_and(|token| match token {
            TokenTree::Literal(_) => true,
            TokenTree::Punct(punct) => punct.as_char() == '-',
            _ => false,
        }) {
            if !seen.insert("status".into()) {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "duplicate problem attribute",
                ));
            }
            let expression = attribute.parse_args::<Expr>()?;
            status = Some(status_number(expression)?);
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            let key = meta.path.get_ident().map(ToString::to_string);
            let Some(key) = key else {
                return Err(meta.error("expected a problem attribute name"));
            };
            if !seen.insert(key.clone()) {
                return Err(meta.error("duplicate problem attribute"));
            }
            match key.as_str() {
                "type_uri" => type_uri = Some(meta.value()?.parse::<LitStr>()?),
                "status" => {
                    let message = STATUS_MESSAGE;
                    let value = meta
                        .value()
                        .map_err(|error| syn::Error::new(error.span(), message))?;
                    let expression = value
                        .parse::<Expr>()
                        .map_err(|error| syn::Error::new(error.span(), message))?;
                    status = Some(status_number(expression)?);
                }
                "title" => title = Some(meta.value()?.parse::<LitStr>()?),
                "detail" => detail = Some(meta.value()?.parse::<LitStr>()?),
                _ => return Err(meta.error("expected type_uri, status, title, or detail")),
            }
            Ok(())
        })?;
    }
    let required =
        |name: &str| syn::Error::new(span, format!("missing problem attribute `{name}`"));
    let type_uri = match (type_uri, prefix) {
        (Some(uri), _) => uri,
        (None, Some(prefix)) => {
            let prefix = prefix.value();
            let separator = if prefix.ends_with([':', '/']) {
                ""
            } else {
                ":"
            };
            let suffix = variant.unraw().to_string().to_kebab_case();
            if suffix.is_empty() {
                return Err(syn::Error::new(
                    variant.span(),
                    "variant name produces an empty problem suffix; set type_uri explicitly",
                ));
            }
            LitStr::new(&format!("{prefix}{separator}{suffix}"), variant.span())
        }
        (None, None) => return Err(required("type_uri")),
    };
    let title = title.unwrap_or_else(|| {
        LitStr::new(&variant.unraw().to_string().to_title_case(), variant.span())
    });
    if type_uri.value().is_empty() || title.value().is_empty() {
        return Err(syn::Error::new(
            span,
            "problem identity and title must not be empty",
        ));
    }
    Ok(Declaration {
        type_uri,
        status,
        title,
        detail,
    })
}
