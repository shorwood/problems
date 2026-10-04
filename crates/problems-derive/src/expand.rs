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
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "Problem derives on enums with unit or named-field variants",
        ));
    };
    let prefix = prefix(&input.attrs)?;
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

    for variant in &data.variants {
        for field in &variant.fields {
            if let Some(attribute) = field.attrs.iter().find(|a| a.path().is_ident("problem")) {
                return Err(syn::Error::new(
                    attribute.span(),
                    "problem attributes belong on variants, not fields",
                ));
            }
        }
        let variant_name = &variant.ident;
        if transparent(&variant.attrs)? {
            let Fields::Unnamed(fields) = &variant.fields else {
                return Err(syn::Error::new(
                    variant.span(),
                    "transparent requires a single-field tuple variant",
                ));
            };
            if fields.unnamed.len() != 1 {
                return Err(syn::Error::new(
                    variant.span(),
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
        if matches!(variant.fields, Fields::Unnamed(_)) {
            return Err(syn::Error::new(
                variant.span(),
                "use unit or named-field problem variants",
            ));
        }
        let declaration = declaration(
            &variant.attrs,
            variant.span(),
            &variant.ident,
            prefix.as_ref(),
        )?;
        if !type_uris.insert(declaration.type_uri.value()) {
            return Err(syn::Error::new(
                declaration.type_uri.span(),
                "duplicate problem type URI; set a distinct type_uri explicitly",
            ));
        }
        let pattern = match variant.fields {
            Fields::Unit => quote!(Self::#variant_name),
            _ => quote!(Self::#variant_name { .. }),
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
        let constant_name = variant_name.unraw().to_string().to_shouty_snake_case();
        if !constant_names.insert(constant_name.clone()) {
            return Err(syn::Error::new(
                variant_name.span(),
                "problem variants generate the same definition constant name",
            ));
        }
        if data
            .variants
            .iter()
            .any(|v| v.ident.unraw() == constant_name)
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
            let fields = detail_fields(&detail)?;
            let mut bindings = Vec::new();
            for (field_name, traits) in fields {
                let field = variant
                    .fields
                    .iter()
                    .find(|field| {
                        field
                            .ident
                            .as_ref()
                            .is_some_and(|ident| ident.unraw() == &field_name)
                    })
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
                bindings.push(field.ident.as_ref().expect("named fields checked above"));
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
            } else {
                quote!(Self::#variant_name { #(#bindings),*, .. })
            };
            detail_arms.push(quote!(#detail_pattern => ::std::option::Option::Some(::std::format!(#detail, #(#bindings = #bindings),*))));
        } else {
            detail_arms.push(quote!(#pattern => ::std::option::Option::None));
        }
    }

    let instance = if data.variants.is_empty() {
        quote!(*self)
    } else {
        quote!(self)
    };
    let (constant_impl_generics, constant_type_generics, constant_where_clause) =
        input.generics.split_for_impl();
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #constant_impl_generics #name #constant_type_generics #constant_where_clause {
            #(#definition_constants)*
        }
        impl #impl_generics #runtime::Problem for #name #type_generics #where_clause {
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
