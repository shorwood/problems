use heck::ToKebabCase;
use proc_macro2::Span;
use std::collections::BTreeSet;
use syn::{Attribute, Expr, ExprPath, Ident, LitStr, ext::IdentExt};

pub(crate) struct Declaration {
    pub(crate) type_uri: LitStr,
    pub(crate) status: Option<ExprPath>,
    pub(crate) title: LitStr,
    pub(crate) detail: Option<LitStr>,
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
                    let message = "status requires a constant path, such as problems::StatusCode::CONFLICT or <Type as Trait>::STATUS; omit status to use 500";
                    let value = meta.value().map_err(|error| {
                        syn::Error::new(error.span(), message)
                    })?;
                    let expression = value.parse::<Expr>().map_err(|error| {
                        syn::Error::new(error.span(), message)
                    })?;
                    let Expr::Path(path) = expression else {
                        return Err(syn::Error::new_spanned(expression, message));
                    };
                    status = Some(path);
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
    let title = title.ok_or_else(|| required("title"))?;
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
