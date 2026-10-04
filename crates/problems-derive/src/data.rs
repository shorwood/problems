use proc_macro2::TokenStream as Tokens;
use quote::{format_ident, quote};
use std::collections::BTreeSet;
use syn::{Data, DeriveInput, Fields, Generics, LitStr, ext::IdentExt, spanned::Spanned};

use crate::{declaration::transparent, fields::is_source};

pub(crate) struct Projection {
    pub declarations: Tokens,
    pub owned: Tokens,
    pub borrowed: Tokens,
    pub borrow_arms: Vec<Tokens>,
    pub move_arms: Vec<Tokens>,
}

/// Build generic payload containers independently of diagnostic field types.
pub(crate) fn projection(
    input: &DeriveInput,
    runtime: &Tokens,
    generics: &mut Generics,
) -> syn::Result<Projection> {
    let is_struct = matches!(input.data, Data::Struct(_));
    let entries = match &input.data {
        Data::Struct(data) => vec![(&input.ident, &input.attrs, &data.fields)],
        Data::Enum(data) => data
            .variants
            .iter()
            .map(|v| (&v.ident, &v.attrs, &v.fields))
            .collect(),
        Data::Union(_) => unreachable!(),
    };
    let name = format_ident!("{}Data", input.ident.unraw());
    let visibility = &input.vis;
    let mut parameters = Vec::new();
    let mut owned_types = Vec::new();
    let mut borrowed_types = Vec::new();
    let mut variants = Vec::new();
    let mut borrow_arms = Vec::new();
    let mut move_arms = Vec::new();
    for (variant, attrs, fields) in entries {
        let constructor = if is_struct {
            quote!(Self)
        } else {
            quote!(Self::#variant)
        };
        let payload = if is_struct {
            quote!(#name)
        } else {
            quote!(#name::#variant)
        };
        let fallback = match fields {
            Fields::Unit => quote!(#constructor),
            Fields::Named(_) => quote!(#constructor { .. }),
            Fields::Unnamed(_) => quote!(#constructor(..)),
        };
        let mut selected = Vec::new();
        let mut keys = BTreeSet::new();
        for (index, field) in fields.iter().enumerate() {
            let mut key = None;
            for attr in field.attrs.iter().filter(|a| a.path().is_ident("problem")) {
                let mut has_data = false;
                attr.parse_nested_meta(|meta| {
                    has_data = true;
                    if !meta.path.is_ident("data") {
                        return Err(meta.error("only data is supported on problem fields"));
                    }
                    if key.is_some() {
                        return Err(meta.error("duplicate public data declaration"));
                    }
                    let value = if meta.input.peek(syn::Token![=]) {
                        meta.value()?.parse::<LitStr>()?
                    } else {
                        let ident = field.ident.as_ref().ok_or_else(|| {
                            meta.error("tuple data fields require an explicit public name")
                        })?;
                        LitStr::new(&ident.unraw().to_string(), ident.span())
                    };
                    if value.value().is_empty() {
                        return Err(syn::Error::new(
                            value.span(),
                            "public data names cannot be empty",
                        ));
                    }
                    key = Some(value);
                    Ok(())
                })?;
                if !has_data {
                    return Err(syn::Error::new(
                        attr.span(),
                        "expected a public data declaration",
                    ));
                }
            }
            if let Some(key) = key {
                if is_source(field) {
                    return Err(syn::Error::new(
                        field.span(),
                        "diagnostic sources cannot be exposed as public data",
                    ));
                }
                if !keys.insert(key.value()) {
                    return Err(syn::Error::new(key.span(), "duplicate public data name"));
                }
                selected.push((index, field, key));
            }
        }
        if transparent(attrs)? {
            if !selected.is_empty() {
                return Err(syn::Error::new(
                    fields.span(),
                    "transparent variants delegate public data",
                ));
            }
            // The main expansion validates transparent shape.
            let Some(field) = fields.iter().next() else {
                continue;
            };
            let ty = &field.ty;
            let parameter = format_ident!("F{}", parameters.len());
            parameters.push(parameter.clone());
            owned_types.push(quote!(<#ty as #runtime::Problem>::Data));
            borrowed_types.push(quote!(<#ty as #runtime::Problem>::DataRef<'__problem_data>));
            variants.push(quote!(#variant(#parameter)));
            borrow_arms.push(
                quote!(#constructor(problem) => #runtime::Problem::data(problem).map(#payload)),
            );
            move_arms.push(quote!(#constructor(problem) => #runtime::Problem::into_data(problem).map(#payload)));
            continue;
        }
        if selected.is_empty() {
            borrow_arms.push(quote!(#fallback => None));
            move_arms.push(quote!(#fallback => None));
            continue;
        }
        let mut members = Vec::new();
        let mut bindings = Vec::new();
        let mut tuple_bindings = vec![quote!(_); fields.len()];
        for (index, field, key) in selected {
            let parameter = format_ident!("F{}", parameters.len());
            let binding = field
                .ident
                .clone()
                .unwrap_or_else(|| format_ident!("field_{index}"));
            let ty = &field.ty;
            generics
                .make_where_clause()
                .predicates
                .push(syn::parse_quote!(#ty: #runtime::__private::serde::Serialize));
            parameters.push(parameter.clone());
            owned_types.push(quote!(#ty));
            borrowed_types.push(quote!(&'__problem_data #ty));
            let vis = if is_struct { quote!(pub) } else { quote!() };
            members.push(quote!(#[serde(rename = #key)] #vis #binding: #parameter));
            tuple_bindings[index] = quote!(#binding);
            bindings.push(binding);
        }
        let pattern = match fields {
            Fields::Named(_) => quote!(#constructor { #(#bindings),*, .. }),
            Fields::Unnamed(_) => quote!(#constructor(#(#tuple_bindings),*)),
            Fields::Unit => unreachable!(),
        };
        variants.push(if is_struct {
            quote!({ #(#members),* })
        } else {
            quote!(#variant { #(#members),* })
        });
        borrow_arms.push(quote!(#pattern => Some(#payload { #(#bindings),* })));
        move_arms.push(quote!(#pattern => Some(#payload { #(#bindings),* })));
    }
    if parameters.is_empty() {
        return Ok(Projection {
            declarations: quote!(),
            owned: quote!(()),
            borrowed: quote!(()),
            borrow_arms,
            move_arms,
        });
    }
    let serde_path = quote!(#runtime::__private::serde)
        .to_string()
        .replace(' ', "");
    let schema_path = quote!(#runtime::__private::schemars)
        .to_string()
        .replace(' ', "");
    let body = if is_struct {
        let fields = &variants[0];
        quote!(#visibility struct #name<#(#parameters),*> #fields)
    } else {
        quote!(#[serde(untagged)] #visibility enum #name<#(#parameters),*> { #(#variants),* })
    };
    Ok(Projection {
        declarations: quote! {
            #runtime::__problem_data! {
                #schema_path,
                /// Explicit public data projected from this problem declaration.
                #[derive(Debug, Clone, PartialEq, Eq, #runtime::__private::serde::Serialize, #runtime::__private::serde::Deserialize)]
                #[serde(crate = #serde_path)]
                #body
            }
        },
        owned: quote!(#name<#(#owned_types),*>),
        borrowed: quote!(#name<#(#borrowed_types),*>),
        borrow_arms,
        move_arms,
    })
}
