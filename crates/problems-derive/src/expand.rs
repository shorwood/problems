use heck::ToShoutySnakeCase;
use proc_macro2::TokenStream as Tokens;
use quote::quote;
use std::collections::BTreeSet;
use syn::{Data, DeriveInput, Fields, Ident, ext::IdentExt, spanned::Spanned};

use crate::{
    declaration::{Declaration, declaration, prefix, transparent},
    detail::detail_fields,
    fields::is_source,
    runtime_path::runtime_path,
};

pub(crate) fn expand(input: DeriveInput) -> syn::Result<Tokens> {
    let runtime = runtime_path()?;
    expand_with_path(input, runtime)
}

pub(crate) fn expand_with_path(input: DeriveInput, runtime: Tokens) -> syn::Result<Tokens> {
    let (entries, prefix, is_struct) = match &input.data {
        Data::Enum(data) => (
            data.variants
                .iter()
                .map(|variant| {
                    (
                        &variant.ident,
                        &variant.attrs,
                        &variant.fields,
                        variant.span(),
                    )
                })
                .collect::<Vec<_>>(),
            prefix(&input.attrs)?,
            false,
        ),
        Data::Struct(data) => (
            vec![(&input.ident, &input.attrs, &data.fields, input.span())],
            None,
            true,
        ),
        Data::Union(_) => {
            return Err(syn::Error::new(
                input.span(),
                "Problem derives on enums or structs",
            ));
        }
    };
    let mut type_uris = BTreeSet::new();
    let name = &input.ident;
    let mut definition_arms = Vec::new();
    let mut definition_constants = Vec::new();
    let mut constant_names = BTreeSet::new();
    let mut detail_arms = Vec::new();
    let mut instance_arms = Vec::new();
    let mut definition_iter = quote!(::std::iter::empty::<&'static #runtime::ProblemDefinition>());
    let mut generics = input.generics.clone();
    generics
        .make_where_clause()
        .predicates
        .push(syn::parse_quote!(Self: ::std::error::Error));

    for &(variant_name, attributes, variant_fields, span) in &entries {
        let constructor = if is_struct {
            quote!(Self)
        } else {
            quote!(Self::#variant_name)
        };
        if transparent(attributes)? {
            if is_struct {
                return Err(syn::Error::new(
                    span,
                    "transparent problem forwarding is supported only on enum variants",
                ));
            }
            let Fields::Unnamed(fields) = variant_fields else {
                return Err(syn::Error::new(
                    span,
                    "transparent requires a single-field tuple variant",
                ));
            };
            if fields.unnamed.len() != 1 {
                return Err(syn::Error::new(
                    span,
                    "transparent requires a single-field tuple variant",
                ));
            }
            let ty = &fields.unnamed[0].ty;
            generics
                .make_where_clause()
                .predicates
                .push(syn::parse_quote!(#ty: #runtime::Problem));
            definition_arms.push(
                quote!(Self::#variant_name(problem) => #runtime::Problem::definition(problem)),
            );
            detail_arms
                .push(quote!(Self::#variant_name(problem) => #runtime::Problem::detail(problem)));
            instance_arms
                .push(quote!(Self::#variant_name(problem) => #runtime::Problem::instance(problem)));
            definition_iter = quote!(::std::iter::Iterator::chain(#definition_iter, <#ty as #runtime::Problem>::definitions()));
            continue;
        }
        let declaration = declaration(attributes, span, variant_name, prefix.as_ref())?;
        if !type_uris.insert(declaration.type_uri.value()) {
            return Err(syn::Error::new(
                declaration.type_uri.span(),
                "duplicate problem type URI; set a distinct type_uri explicitly",
            ));
        }
        let pattern = match variant_fields {
            Fields::Unit => quote!(#constructor),
            Fields::Named(_) => quote!(#constructor { .. }),
            Fields::Unnamed(_) => quote!(#constructor(..)),
        };
        let Declaration {
            type_uri,
            status,
            title,
            detail,
        } = declaration;
        let status = status.map_or_else(
            || quote!(#runtime::StatusCode::INTERNAL_SERVER_ERROR),
            |number| quote!(const {
                match #runtime::StatusCode::from_u16(#number) {
                    ::std::result::Result::Ok(status) => status,
                    ::std::result::Result::Err(_) => panic!("status validated by Problem derive"),
                }
            }),
        );
        let definition = quote!(#runtime::ProblemDefinition {
            type_uri: #type_uri,
            status: #status,
            title: #title,
        });
        let constant_name = if is_struct {
            "DEFINITION".to_owned()
        } else {
            variant_name.unraw().to_string().to_shouty_snake_case()
        };
        if !constant_names.insert(constant_name.clone()) {
            return Err(syn::Error::new(
                variant_name.span(),
                "problem variants generate the same definition constant name",
            ));
        }
        if !is_struct
            && entries
                .iter()
                .any(|(ident, _, _, _)| ident.unraw() == constant_name)
        {
            return Err(syn::Error::new(
                variant_name.span(),
                "generated problem definition constant conflicts with an enum variant",
            ));
        }
        let constant = Ident::new(&constant_name, variant_name.span());
        let documentation = format!("Public problem definition for `{variant_name}`.");
        definition_constants.push(quote! {
            #[doc = #documentation]
            pub const #constant: #runtime::ProblemDefinition = #definition;
        });
        definition_arms.push(quote!(#pattern => &Self::#constant));
        instance_arms.push(quote!(#pattern => ::std::option::Option::None));
        definition_iter = quote!(::std::iter::Iterator::chain(#definition_iter, ::std::iter::once(&Self::#constant)));

        if let Some(detail) = detail {
            let tuple = matches!(variant_fields, Fields::Unnamed(_));
            let (detail, fields) = detail_fields(&detail, tuple)?;
            let mut bindings = Vec::new();
            let mut tuple_bindings = vec![quote!(_); variant_fields.len()];
            for (field_name, traits) in fields {
                let field = if tuple {
                    variant_fields
                        .iter()
                        .nth(field_name.parse::<usize>().unwrap())
                } else {
                    variant_fields.iter().find(|field| {
                        field
                            .ident
                            .as_ref()
                            .is_some_and(|ident| ident.unraw() == &field_name)
                    })
                }
                .ok_or_else(|| {
                    syn::Error::new(
                        detail.span(),
                        format!("unknown detail field `{field_name}`"),
                    )
                })?;
                if is_source(field) {
                    return Err(syn::Error::new(
                        detail.span(),
                        "diagnostic sources cannot be formatted into public detail",
                    ));
                }
                let binding = if tuple {
                    let binding =
                        Ident::new(&format!("__problem_field_{field_name}"), detail.span());
                    tuple_bindings[field_name.parse::<usize>().unwrap()] = quote!(#binding);
                    binding
                } else {
                    field.ident.clone().expect("named fields checked above")
                };
                bindings.push(binding);
                let ty = &field.ty;
                for trait_name in traits {
                    let format_trait = Ident::new(trait_name, detail.span());
                    generics
                        .make_where_clause()
                        .predicates
                        .push(syn::parse_quote!(#ty: ::std::fmt::#format_trait));
                }
            }
            let detail_pattern = if bindings.is_empty() {
                pattern.clone()
            } else if tuple {
                quote!(#constructor(#(#tuple_bindings),*))
            } else {
                quote!(#constructor { #(#bindings),*, .. })
            };
            detail_arms.push(quote!(#detail_pattern => ::std::option::Option::Some(::std::format!(#detail, #(#bindings = #bindings),*))));
        } else {
            detail_arms.push(quote!(#pattern => ::std::option::Option::None));
        }
    }

    let crate::data::Projection {
        declarations,
        owned,
        borrowed,
        borrow_arms,
        move_arms,
    } = crate::data::projection(&input, &runtime, &mut generics)?;

    let instance = if entries.is_empty() {
        quote!(*self)
    } else {
        quote!(self)
    };
    let (constant_impl_generics, constant_type_generics, constant_where_clause) =
        input.generics.split_for_impl();
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        #declarations
        impl #constant_impl_generics #name #constant_type_generics #constant_where_clause {
            #(#definition_constants)*
        }
        impl #impl_generics #runtime::Problem for #name #type_generics #where_clause {
            type Data = #owned;
            type DataRef<'__problem_data> = #borrowed where Self: '__problem_data;
            fn data(&self) -> Option<Self::DataRef<'_>> {
                match #instance { #(#borrow_arms),* }
            }
            fn into_data(self) -> Option<Self::Data> {
                match self { #(#move_arms),* }
            }
            fn definition(&self) -> &'static #runtime::ProblemDefinition {
                match #instance { #(#definition_arms),* }
            }
            fn definitions() -> impl ::std::iter::Iterator<Item = &'static #runtime::ProblemDefinition> {
                #definition_iter
            }
            fn detail(&self) -> ::std::option::Option<::std::string::String> {
                match #instance { #(#detail_arms),* }
            }
            fn instance(&self) -> ::std::option::Option<::std::string::String> {
                match #instance { #(#instance_arms),* }
            }
        }
    })
}
