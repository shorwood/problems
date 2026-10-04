use proc_macro2::{Span, TokenStream as Tokens};
use quote::quote;
use syn::Ident;

pub(crate) fn runtime_path() -> syn::Result<Tokens> {
    match proc_macro_crate::crate_name("problems")
        .map_err(|error| syn::Error::new(Span::call_site(), error))?
    {
        proc_macro_crate::FoundCrate::Itself => Ok(quote!(::problems)),
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            Ok(quote!(::#ident))
        }
    }
}
