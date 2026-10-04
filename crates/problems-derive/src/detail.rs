use std::collections::{BTreeMap, BTreeSet};
use syn::{Ident, LitStr};

// Extract explicit named arguments and their formatting traits. Rust's format!
// remains responsible for validating format specifiers; this scanner validates
// field ownership and keeps unreferenced fields out of generated match arms.
pub(crate) fn detail_fields(
    detail: &LitStr,
) -> syn::Result<BTreeMap<String, BTreeSet<&'static str>>> {
    let text = detail.value();
    let mut chars = text.chars().peekable();
    let mut fields = BTreeMap::<String, BTreeSet<&'static str>>::new();
    while let Some(character) = chars.next() {
        match character {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
            }
            '{' => {
                let mut placeholder = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some('{') | None => {
                            return Err(syn::Error::new(
                                detail.span(),
                                "invalid detail format string",
                            ));
                        }
                        Some(character) => placeholder.push(character),
                    }
                }
                let (name, specifier) = placeholder.split_once(':').unwrap_or((&placeholder, ""));
                syn::parse_str::<Ident>(name).map_err(|_| {
                    syn::Error::new(
                        detail.span(),
                        "detail formatting requires named variant fields",
                    )
                })?;
                let trait_name = match specifier.chars().last() {
                    Some('?') => "Debug",
                    Some('x') => "LowerHex",
                    Some('X') => "UpperHex",
                    Some('o') => "Octal",
                    Some('b') => "Binary",
                    Some('p') => "Pointer",
                    Some('e') => "LowerExp",
                    Some('E') => "UpperExp",
                    _ => "Display",
                };
                fields.entry(name.into()).or_default().insert(trait_name);
            }
            '}' => {
                return Err(syn::Error::new(
                    detail.span(),
                    "unmatched closing brace in detail",
                ));
            }
            _ => {}
        }
    }
    Ok(fields)
}
