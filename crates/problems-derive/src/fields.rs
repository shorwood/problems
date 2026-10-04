use proc_macro2::{TokenStream as Tokens, TokenTree};
use quote::ToTokens;

fn has_source_token(tokens: Tokens) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident == "source",
        TokenTree::Group(group) => has_source_token(group.stream()),
        _ => false,
    })
}

pub(crate) fn is_source(field: &syn::Field) -> bool {
    field.ident.as_ref().is_some_and(|name| name == "source")
        || field.attrs.iter().any(|attribute| {
            attribute.path().is_ident("source")
                || attribute.path().is_ident("from")
                || (attribute.path().is_ident("error")
                    && has_source_token(attribute.meta.to_token_stream()))
        })
}
