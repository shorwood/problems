//! Derive public problem metadata while leaving ordinary Rust errors intact.

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod declaration;
mod detail;
mod expand;
mod fields;
mod runtime_path;

/// Declare public problems on enum variants.
///
/// Every variant requires a nonempty string-literal `title`. Braces in titles
/// are literal text, including `{name}`; interpolation belongs in `detail`.
/// An enum's `#[problem(prefix = "...")]`
/// generates type URIs from kebab-case variant names; an explicit variant
/// `type_uri` overrides it. Without a prefix, every variant requires `type_uri`.
/// A colon separates prefix and name unless the prefix ends in `:` or `/`.
/// Renaming a variant changes its generated URI; use an explicit URI to preserve
/// an existing public identity. Duplicate type URIs are rejected. Optional `status`
/// accepts a constant path and defaults to 500. Optional `detail` supports Rust
/// named-field formatting. Other fields remain diagnostic.
#[proc_macro_derive(Problem, attributes(problem))]
pub fn derive_problem(input: TokenStream) -> TokenStream {
    expand::expand(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
