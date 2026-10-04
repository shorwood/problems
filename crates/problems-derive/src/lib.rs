//! Derive public problem metadata while leaving ordinary Rust errors intact.

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod data;
mod declaration;
mod detail;
mod expand;
mod fields;
mod runtime_path;

/// Declare public problems on enum variants or a single struct.
///
/// Struct declarations require an explicit `type_uri` and expose `DEFINITION`.
/// Unit, named-field, and tuple structs follow the same formatting and source
/// protection rules as enum variants.
///
/// Titles default to the variant or struct name converted to Title Case by `heck`.
/// An explicit `title` must be a nonempty string literal. Braces in titles
/// are literal text, including `{name}`; interpolation belongs in `detail`.
/// An enum's `#[problem(prefix = "...")]`
/// generates type URIs from kebab-case variant names; an explicit variant
/// `type_uri` overrides it. Without a prefix, every variant requires `type_uri`.
/// A colon separates prefix and name unless the prefix ends in `:` or `/`.
/// Renaming a variant changes its generated URI; use an explicit URI to preserve
/// an existing public identity. Duplicate type URIs are rejected. Optional `status`
/// accepts an integer literal from 100 through 999 and defaults to 500.
/// `#[problem(409)]` is shorthand for `#[problem(status = 409)]`; use a separate
/// attribute for other declarations. Both forms share validation and duplicate checks.
/// Optional `detail` supports named fields or explicit tuple indexes with literal width
/// and precision; dynamic formatting
/// parameters are unsupported. Other fields remain diagnostic.
/// Select named public fields with `#[problem(data)]`; use
/// `#[problem(data = "name")]` to rename or select tuple fields.
/// Sources cannot be selected. Generated `<Type>Data` containers support
/// borrowing and moving without requiring selected fields to implement `Clone`.
#[proc_macro_derive(Problem, attributes(problem))]
pub fn derive_problem(input: TokenStream) -> TokenStream {
    expand::expand(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
