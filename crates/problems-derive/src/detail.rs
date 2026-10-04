use std::collections::{BTreeMap, BTreeSet};
use syn::{Ident, LitStr, ext::IdentExt, parse::Parser};

// Extract explicit field arguments and their formatting traits. Rust's format!
// remains responsible for validating format specifiers; this scanner validates
// field ownership and keeps unreferenced fields out of generated match arms.
// Tuple indexes become named arguments so sparse references need no unused args.
pub(crate) fn detail_fields(
    detail: &LitStr,
    tuple: bool,
) -> syn::Result<(LitStr, BTreeMap<String, BTreeSet<&'static str>>)> {
    let text = detail.value();
    let mut chars = text.chars().peekable();
    let mut rewritten = String::new();
    let mut fields = BTreeMap::<String, BTreeSet<&'static str>>::new();
    while let Some(character) = chars.next() {
        match character {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                rewritten.push(character);
                rewritten.push(character);
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                rewritten.push(character);
                rewritten.push(character);
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
                let argument = if tuple {
                    if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_digit()) {
                        return Err(syn::Error::new(
                            detail.span(),
                            "tuple detail requires explicit field indexes, such as {0}",
                        ));
                    }
                    let index = name.parse::<usize>().map_err(|_| {
                        syn::Error::new(detail.span(), "tuple detail field index is out of range")
                    })?;
                    format!("__problem_field_{index}")
                } else {
                    Ident::parse_any.parse_str(name).map_err(|_| {
                        syn::Error::new(
                            detail.span(),
                            "detail formatting requires named variant fields",
                        )
                    })?;
                    name.to_owned()
                };
                rewritten.push('{');
                rewritten.push_str(&argument);
                if placeholder.contains(':') {
                    rewritten.push(':');
                    rewritten.push_str(specifier);
                }
                rewritten.push('}');
                // A fill character followed by alignment is literal, even `$` or `*`.
                let mut parameters = specifier.chars();
                let mut prefix = parameters.clone();
                if prefix.next().is_some()
                    && prefix.next().is_some_and(|c| matches!(c, '<' | '^' | '>'))
                {
                    parameters.next();
                    parameters.next();
                }
                let parameters = parameters.as_str();
                if parameters.contains('$') || parameters.contains(".*") {
                    return Err(syn::Error::new(
                        detail.span(),
                        "dynamic width and precision are not supported in detail; use named variant fields with literal width and precision",
                    ));
                }
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
                fields
                    .entry(if tuple {
                        name.parse::<usize>().unwrap().to_string()
                    } else {
                        name.into()
                    })
                    .or_default()
                    .insert(trait_name);
            }
            '}' => {
                return Err(syn::Error::new(
                    detail.span(),
                    "unmatched closing brace in detail",
                ));
            }
            _ => rewritten.push(character),
        }
    }
    Ok((LitStr::new(&rewritten, detail.span()), fields))
}
