//! Generate the public problem contract for Rust errors.
//!
//! This crate implements the derive re-exported by `problems`. Applications
//! normally use `#[derive(problems::Problem)]` alongside an implementation of
//! `Display` and `Error`, such as `thiserror::Error`. The derive selects public
//! metadata, detail, and data; the original error keeps its diagnostic message
//! and source chain.
//!
//! Expansion follows three stages:
//!
//! 1. Read the struct or enum variants, validate their metadata, and resolve
//!    detail placeholders against their fields.
//! 2. Select and validate the structured public data fields.
//! 3. Generate definition constants, a data container when needed, and the
//!    runtime `Problem` implementation. Invalid input produces compiler errors
//!    instead of a partial implementation.
//!
//! See [`macro@Problem`] for the supported attributes and examples.

use heck::{ToKebabCase, ToShoutySnakeCase, ToTitleCase};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as Tokens, TokenTree};
use quote::{ToTokens, format_ident, quote};
use std::collections::{BTreeMap, BTreeSet};
use syn::{
    Attribute, Data, DeriveInput, Expr, Field, Fields, Ident, Lit, LitStr, Type,
    ext::IdentExt,
    parse::{ParseStream, Parser},
    parse_macro_input,
};

/****************************************/
/* Derive Entry Point                   */
/****************************************/

/// Derive the public metadata and payload of an error struct or enum.
///
/// The error must also implement `std::error::Error`. This derive does not
/// implement diagnostic formatting or copy the diagnostic message into the
/// public detail. Applications normally import this macro through `problems`.
///
/// # Declare a public problem
///
/// ```rust
/// use issues::{IntoReport, StatusCode};
///
/// // --- Declare diagnostics separately from the public contract.
/// #[derive(Debug, thiserror::Error, problems_derive::Problem)]
/// #[problem(prefix = "urn:example")]
/// enum CreateProblem {
///     #[error("duplicate name: {name}")]
///     #[problem(status = 409, detail = "The name '{name}' is already in use.")]
///     NameConflict {
///         #[problem(data)]
///         name: String,
///     },
/// }
///
/// // --- Borrow the public payload while keeping the error available.
/// let report = CreateProblem::NameConflict { name: "monthly".into() }.into_report();
/// let details = report.as_details();
/// assert_eq!(details.status(), StatusCode::CONFLICT);
/// assert_eq!(details.type_uri(), "urn:example:name-conflict");
/// assert_eq!(details.detail(), Some("The name 'monthly' is already in use."));
/// assert_eq!(serde_json::to_value(&details)?["data"]["name"], "monthly");
/// assert_eq!(report.problem().to_string(), "duplicate name: monthly");
///
/// // --- Consume the report when the document must own its payload.
/// let details = report.into_details();
/// assert_eq!(serde_json::to_value(&details)?["data"]["name"], "monthly");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// The runtime dependency is named `issues` in these examples to demonstrate
/// support for a renamed `problems` dependency.
///
/// # Public identity and status
///
/// Put these attributes on a struct or on each nontransparent enum variant:
///
/// - `type_uri = "..."` supplies a nonempty public identifier. URI syntax is not
///   validated. A struct always needs it; an enum variant can inherit a prefix.
/// - `title = "..."` supplies a nonempty summary. Otherwise, the declaration name
///   becomes Title Case, such as `Name Conflict`.
/// - `status = 409` supplies an integer literal from 100 through 999. The default
///   is 500. A separate `#[problem(409)]` attribute provides the shorthand.
///
/// On an enum, `#[problem(prefix = "urn:example")]` derives each local variant's
/// URI from its kebab-case name. Prefixes ending in `:` or `/` need no separator;
/// other prefixes get `:`. An explicit variant URI overrides the prefix. Set an
/// explicit URI when a Rust rename must preserve the public identifier.
///
/// A struct receives a `DEFINITION` associated constant. An enum receives a
/// constant per local variant in SHOUTY_SNAKE_CASE, such as `NAME_CONFLICT`.
/// Duplicate local URIs, duplicate constant names, and constants that collide
/// with enum variant names are rejected.
///
/// # Public detail and data
///
/// `detail = "..."` formats named fields, such as `{name}`, or explicit tuple
/// indexes, such as `{0}`. Without this attribute, the local detail is absent.
/// Literal braces use `{{` and `}}`. Formatting traits follow the specifier,
/// such as `Debug` for `{name:?}`. Literal width and precision are supported;
/// dynamic width and precision are rejected. Rust checks the remaining format
/// syntax when it compiles the generated `format!` call.
/// Local definitions have no instance value; transparent variants forward it.
///
/// `#[problem(data)]` selects a named field for the structured payload.
/// `#[problem(data = "public_name")]` sets its serialized name and is required
/// for tuple fields. Selected names must be nonempty and unique per declaration.
/// Selecting data and interpolating detail are independent choices.
///
/// Fields named `source`, marked `#[source]` or `#[from]`, or containing a
/// `source` token in an `#[error(...)]` attribute are treated as diagnostic
/// sources. They cannot appear in public detail or data.
///
/// Selected fields generate a `<Type>Data` container with generic parameters for
/// field types. Borrowing fills it with references; consuming moves the fields.
/// Generated enum payloads are Serde-untagged, with no variant tag. Declarations
/// without data return no payload; a type with no data uses `()` for its payload
/// types. Generated containers follow the input type's visibility and use the
/// runtime crate's Serde and optional schema support.
///
/// # Forward an existing problem
///
/// Only a single-field tuple enum variant can use `#[problem(transparent)]`.
/// It forwards definition, detail, instance, and data to the inner problem.
/// It accepts no other problem metadata or selected data fields and generates
/// no local definition constant. Its inner definitions join the enclosing
/// list in variant order.
///
/// ```rust
/// use issues::Problem;
///
/// // --- Give the inner error its own public identity.
/// #[derive(Debug, thiserror::Error, problems_derive::Problem)]
/// #[error("record not found")]
/// #[problem(type_uri = "urn:example:missing-record", status = 404)]
/// struct MissingRecord;
///
/// // --- Reuse that contract in an enclosing error enum.
/// #[derive(Debug, thiserror::Error, problems_derive::Problem)]
/// enum RequestProblem {
///     #[error(transparent)]
///     #[problem(transparent)]
///     Missing(MissingRecord),
/// }
///
/// // --- The wrapper exposes the same definition and definition list.
/// let problem = RequestProblem::Missing(MissingRecord);
/// assert_eq!(problem.definition(), &MissingRecord::DEFINITION);
/// assert_eq!(RequestProblem::definitions().collect::<Vec<_>>(),
///            vec![&MissingRecord::DEFINITION]);
/// ```
///
/// # Invalid declarations
///
/// Errors identify the attribute, literal, field, or declaration that needs a
/// change. Conflicts also point to the first declaration. Independent variants
/// can report errors together: metadata errors come first, followed by data
/// errors for declarations whose metadata was valid. Parsing stops at the first
/// error within a declaration; unsupported input shapes, invalid enum prefixes,
/// and an unavailable runtime crate stop the whole expansion.
#[proc_macro_derive(Problem, attributes(problem))]
pub fn derive_problem(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_problem(&input)
        .unwrap_or_else(MacroError::into_tokens)
        .into()
}

fn expand_problem(input: &DeriveInput) -> MacroResult<Tokens> {
    let runtime = resolve_runtime_crate()?;
    let problem = ProblemDerive::parse(input)?;

    Ok(problem.generate(&runtime))
}

/****************************************/
/* Compiler Diagnostics                 */
/****************************************/

/// Domain errors shared by attribute parsing and validation.
///
/// Messages live here; the impl adds actionable help, context, and source spans.
#[derive(Debug, thiserror::Error)]
enum ProblemDiagnostic {
    #[error("Problem derives on enums or structs")]
    UnsupportedShape,
    #[error("transparent problem forwarding is supported only on enum variants")]
    TransparentStruct,
    #[error("transparent requires a single-field tuple variant")]
    TransparentShape,
    #[error("transparent cannot be combined with other problem attributes")]
    TransparentAttributes,
    #[error("transparent variants delegate public data")]
    TransparentData,
    #[error("missing problem attribute `type_uri`")]
    MissingTypeUri { is_struct: bool },
    #[error("variant name produces an empty problem suffix")]
    EmptyTypeSuffix,
    #[error("problem type URI must not be empty")]
    EmptyTypeUri,
    #[error("problem title must not be empty")]
    EmptyTitle,
    #[error("problem prefix must not be empty")]
    EmptyPrefix,
    #[error("public data names cannot be empty")]
    EmptyDataName,
    #[error("expected nested attribute")]
    EmptyAttribute,
    #[error("expected a problem attribute name")]
    InvalidAttributeName,
    #[error("unsupported problem declaration attribute")]
    UnknownDeclarationAttribute,
    #[error("unsupported problem attribute on the enum")]
    UnknownEnumAttribute,
    #[error("unsupported problem attribute on the field")]
    UnknownFieldAttribute,
    #[error("duplicate problem attribute `{name}`")]
    DuplicateAttribute { name: &'static str },
    #[error("duplicate problem type URI `{uri}`")]
    DuplicateTypeUri { uri: String },
    #[error("problem variants generate the same definition constant name `{name}`")]
    DuplicateConstant { name: String },
    #[error("generated problem definition constant `{name}` conflicts with an enum variant")]
    VariantConstantCollision { name: String },
    #[error("duplicate public data name `{name}`")]
    DuplicateDataName { name: String },
    #[error("problem status must be an integer literal from 100 through 999")]
    InvalidStatus,
    #[error("tuple data fields require an explicit public name")]
    TupleDataNameRequired,
    #[error("diagnostic sources cannot be exposed as public data")]
    DiagnosticSourceData,
    #[error("diagnostic sources cannot be formatted into public detail")]
    DiagnosticSourceDetail,
    #[error("unmatched closing brace in detail")]
    UnmatchedDetailBrace,
    #[error("invalid detail format string")]
    InvalidDetailFormat,
    #[error("dynamic width and precision are not supported in `detail`")]
    DynamicDetailFormat,
    #[error("tuple detail requires explicit field indexes")]
    TupleDetailIndexRequired,
    #[error("tuple detail field index is out of range")]
    TupleDetailIndexOutOfRange,
    #[error("detail formatting requires named variant fields")]
    NamedDetailFieldRequired,
    #[error("unknown detail field `{name}`")]
    UnknownDetailField { name: String, available: String },
    #[error("first declared here")]
    FirstDeclaration,
}

impl ProblemDiagnostic {
    fn at(self, tokens: impl ToTokens) -> MacroError {
        syn::Error::new_spanned(tokens, self.message()).into()
    }

    fn at_span(self, span: Span) -> MacroError {
        syn::Error::new(span, self.message()).into()
    }

    /// Report the conflict and preserve the original declaration as a second location.
    fn conflict(self, span: Span, first: Span) -> MacroError {
        let mut error = syn::Error::new(span, self.message());
        error.combine(syn::Error::new(first, Self::FirstDeclaration));
        error.into()
    }

    /// Attach help and notes before rendering the diagnostic into compiler errors.
    fn message(&self) -> String {
        let mut message = self.to_string();
        let help = self.help();
        let note = self.note();
        if help.is_some() || note.is_some() {
            message.push_str("\n\n");
            for (kind, text) in [("help", help), ("note", note.as_deref())] {
                if let Some(text) = text {
                    message.push_str(&format!("  = {kind}: {text}\n"));
                }
            }
            message.push('\n');
        }
        message
    }

    fn help(&self) -> Option<&'static str> {
        Some(match self {
            Self::UnsupportedShape => "derive `Problem` on a struct or enum instead of a union",
            Self::TransparentStruct => {
                "declare an enum variant such as `Wrapped(Inner)` with `#[problem(transparent)]`"
            }
            Self::TransparentShape => {
                "use a tuple variant containing exactly one field, such as `Wrapped(Inner)`"
            }
            Self::TransparentAttributes => {
                "keep only `#[problem(transparent)]` on this variant; configure metadata on the inner problem"
            }
            Self::TransparentData => {
                "remove the field's `#[problem(data)]` attribute; select public data on the inner problem"
            }
            Self::MissingTypeUri { is_struct: true } => {
                "add `#[problem(type_uri = \"urn:example:failure\")]` to this struct"
            }
            Self::MissingTypeUri { is_struct: false } => {
                "add `#[problem(type_uri = \"urn:example:failure\")]` to this declaration, or set `#[problem(prefix = \"urn:example\")]` on its enum"
            }
            Self::EmptyTypeSuffix => "set a nonempty `type_uri` explicitly on this variant",
            Self::EmptyTypeUri => "provide a nonempty `type_uri`, such as \"urn:example:failure\"",
            Self::EmptyTitle => {
                "provide a nonempty title, or omit `title` to derive it from the declaration name"
            }
            Self::EmptyPrefix => {
                "provide a nonempty prefix, or omit `prefix` and specify `type_uri` on every variant"
            }
            Self::EmptyDataName => {
                "provide a nonempty serialized name, or use `#[problem(data)]` on a named field"
            }
            Self::EmptyAttribute | Self::InvalidAttributeName => {
                "use `#[problem(name = value)]`; supported names depend on whether the attribute is on an enum, declaration, or field"
            }
            Self::UnknownDeclarationAttribute => {
                "use `type_uri`, `status`, `title`, or `detail` on a struct or enum variant"
            }
            Self::UnknownEnumAttribute => {
                "use `#[problem(prefix = \"urn:example\")]` on the enum; put `type_uri`, `status`, `title`, and `detail` on its variants"
            }
            Self::UnknownFieldAttribute => {
                "use `#[problem(data)]` or `#[problem(data = \"public_name\")]` to select a field"
            }
            Self::DuplicateAttribute { .. } => "keep exactly one declaration of this attribute",
            Self::DuplicateTypeUri { .. } => {
                "set a distinct `type_uri` explicitly on one of these declarations"
            }
            Self::DuplicateConstant { .. } | Self::VariantConstantCollision { .. } => {
                "rename a variant so its generated definition constant has a distinct name"
            }
            Self::DuplicateDataName { .. } => {
                "give the selected fields distinct serialized names using `#[problem(data = \"public_name\")]`"
            }
            Self::InvalidStatus => {
                "use `#[problem(status = 409)]` or `#[problem(409)]` with an integer literal"
            }
            Self::TupleDataNameRequired => {
                "select this tuple field with `#[problem(data = \"public_name\")]`"
            }
            Self::DiagnosticSourceData => {
                "remove `#[problem(data)]` from the source field; select a separate public field instead"
            }
            Self::DiagnosticSourceDetail => {
                "remove the source field's placeholder from `detail`; format a separate public field instead"
            }
            Self::UnmatchedDetailBrace | Self::InvalidDetailFormat => {
                "close each placeholder with `}`; escape literal braces as `{{` and `}}`"
            }
            Self::DynamicDetailFormat => {
                "use literal width and precision, such as `{value:8.2}` for named fields or `{0:8.2}` for tuple fields"
            }
            Self::TupleDetailIndexRequired => {
                "reference tuple fields by zero-based index, such as `detail = \"failed: {0}\"`"
            }
            Self::TupleDetailIndexOutOfRange => {
                "use a zero-based index that identifies an existing tuple field"
            }
            Self::NamedDetailFieldRequired => {
                "reference a named field, such as `detail = \"failed: {value}\"`"
            }
            Self::UnknownDetailField { .. } => {
                "use a field that can appear in public detail, or escape literal braces as `{{` and `}}`"
            }
            Self::FirstDeclaration => return None,
        })
    }

    fn note(&self) -> Option<String> {
        let note = match self {
            Self::TransparentStruct
            | Self::TransparentShape
            | Self::TransparentAttributes
            | Self::TransparentData => {
                "transparent variants forward metadata, detail, instance, and public data to their inner `Problem`"
            }
            Self::DuplicateConstant { .. } | Self::VariantConstantCollision { .. } => {
                "definition constant names are generated from variant names in SHOUTY_SNAKE_CASE"
            }
            Self::InvalidStatus => {
                "omitting `status` uses 500; constants, expressions, and string literals are not supported"
            }
            Self::DiagnosticSourceData | Self::DiagnosticSourceDetail => {
                "diagnostic sources are kept out of public problem details"
            }
            Self::UnknownDetailField { available, .. } => return Some(available.clone()),
            _ => return None,
        };
        Some(note.to_owned())
    }
}

type MacroResult<T> = std::result::Result<T, MacroError>;

/// Compiler-error tokens from either Syn parsing or a domain diagnostic.
///
/// Both paths preserve their source spans and use the same expansion result.
/// Help and notes are rendered inside the error message; they do not require
/// nightly compiler diagnostics.
struct MacroError(Tokens);

impl MacroError {
    fn into_tokens(self) -> Tokens {
        self.0
    }
}

impl From<syn::Error> for MacroError {
    fn from(error: syn::Error) -> Self {
        Self(error.into_compile_error())
    }
}

/// Collect independent errors without emitting a partial implementation.
#[derive(Default)]
struct Diagnostics {
    errors: Tokens,
}

impl Diagnostics {
    fn push(&mut self, error: MacroError) {
        self.errors.extend(error.into_tokens());
    }

    /// Keep valid values for later validation and collect errors from invalid ones.
    fn handle<T>(&mut self, result: MacroResult<T>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.push(error);
                None
            }
        }
    }

    /// Allow generation only when every collected validation has succeeded.
    fn finish(self) -> MacroResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(MacroError(self.errors))
        }
    }
}

/****************************************/
/* Declaration Model                    */
/****************************************/

/// Validated expansion input, borrowing the original syntax to retain types and spans.
struct ProblemDerive<'a> {
    input: &'a DeriveInput,
    declarations: Vec<Declaration<'a>>,
}

/// One struct or enum variant, with its public contract and selected payload.
struct Declaration<'a> {
    syntax: DeclarationSyntax<'a>,
    kind: DeclarationKind<'a>,
    public_data: PublicData<'a>,
}

/// A local definition or forwarding to an existing inner problem.
enum DeclarationKind<'a> {
    Defined(ProblemMetadata<'a>),
    Transparent(&'a Type),
}

/// Resolved metadata for a local definition, including any validated detail format.
struct ProblemMetadata<'a> {
    constant_name: Ident,
    type_uri: LitStr,
    title: LitStr,
    status: u16,
    detail: Option<DetailFormat<'a>>,
}

/// A rewritten format string and the fields and formatting bounds it requires.
struct DetailFormat<'a> {
    format_string: LitStr,
    arguments: Vec<DetailArgument<'a>>,
}

/// One referenced field, collecting every formatting trait its placeholders require.
struct DetailArgument<'a> {
    selection: SelectedField<'a>,
    format_traits: BTreeSet<Ident>,
}

/// The payload strategy: absent, selected local fields, or an inner problem payload.
enum PublicData<'a> {
    None,
    Fields(Vec<DataField<'a>>),
    Transparent { ty: &'a Type, parameter: Ident },
}

/// A selected input field with its serialized name and container type parameter.
struct DataField<'a> {
    selection: SelectedField<'a>,
    serialized_name: LitStr,
    parameter: Ident,
}

/// Locate an input field and give it a binding usable in generated match arms.
///
/// Tuple bindings use `field_<index>`; indexes always refer to the original fields.
struct SelectedField<'a> {
    index: usize,
    binding: Ident,
    ty: &'a Type,
}

impl<'a> SelectedField<'a> {
    fn new(index: usize, field: &'a Field) -> Self {
        let binding = field
            .ident
            .clone()
            .unwrap_or_else(|| format_ident!("field_{index}"));
        Self {
            index,
            binding,
            ty: &field.ty,
        }
    }

    fn pattern_binding(&self) -> (usize, Ident) {
        (self.index, self.binding.clone())
    }
}

/// Shared view of struct and variant syntax for validation and match generation.
struct DeclarationSyntax<'a> {
    name: &'a Ident,
    attributes: &'a [Attribute],
    fields: &'a Fields,
    is_struct: bool,
}

/****************************************/
/* Declaration Validation               */
/****************************************/

impl<'a> ProblemDerive<'a> {
    /// Resolve metadata before selecting public data; collect errors across declarations.
    fn parse(input: &'a DeriveInput) -> MacroResult<Self> {
        let syntaxes = DeclarationSyntax::collect(input)?;
        let prefix = match input.data {
            Data::Enum(_) => input.attrs.parse_prefix()?,
            _ => None,
        };
        let mut type_uris = BTreeMap::new();
        let mut constant_names = BTreeMap::new();
        let mut declarations = Vec::new();
        let mut diagnostics = Diagnostics::default();

        // --- Validate each public identity and resolve its detail fields.
        for syntax in syntaxes {
            let parsed = (|| {
                let kind = if syntax.attributes.is_transparent()? {
                    DeclarationKind::Transparent(syntax.transparent_type()?)
                } else {
                    let attributes = ProblemAttributes::read(syntax.attributes)?;
                    let constant_name = syntax.constant_name();
                    let type_uri = syntax.resolve_type_uri(attributes.type_uri, prefix.as_ref())?;
                    let title = syntax.resolve_title(attributes.title);
                    syntax.check_identity(&type_uri, &title)?;
                    ProblemMetadata::check_unique_uri(&type_uri, &mut type_uris)?;
                    syntax.check_unique_constant(&constant_name, &mut constant_names)?;
                    syntax.check_variant_collision(input, &constant_name)?;
                    let detail = attributes
                        .detail
                        .map(|text| DetailFormat::parse(&text, syntax.fields))
                        .transpose()?;
                    DeclarationKind::Defined(ProblemMetadata {
                        constant_name,
                        type_uri,
                        title,
                        status: attributes.status.unwrap_or(500),
                        detail,
                    })
                };
                Ok(Declaration {
                    syntax,
                    kind,
                    public_data: PublicData::None,
                })
            })();
            if let Some(declaration) = diagnostics.handle(parsed) {
                declarations.push(declaration);
            }
        }

        // --- Select data only for declarations whose metadata was valid.
        let mut next_parameter = 0;
        for declaration in &mut declarations {
            if let Some(public_data) =
                diagnostics.handle(declaration.parse_public_data(&mut next_parameter))
            {
                declaration.public_data = public_data;
            }
        }
        // --- Generation must never receive a partially valid model.
        diagnostics.finish()?;
        Ok(Self {
            input,
            declarations,
        })
    }
}

impl<'a> DeclarationSyntax<'a> {
    fn transparent_type(&self) -> MacroResult<&'a Type> {
        if self.is_struct {
            return Err(ProblemDiagnostic::TransparentStruct.at(self.name));
        }
        match self.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(&fields.unnamed[0].ty),
            _ => Err(ProblemDiagnostic::TransparentShape.at(self.name)),
        }
    }

    fn constant_name(&self) -> Ident {
        let name = if self.is_struct {
            "DEFINITION".to_owned()
        } else {
            self.name.unraw().to_string().to_shouty_snake_case()
        };
        Ident::new(&name, self.name.span())
    }

    fn resolve_type_uri(
        &self,
        explicit: Option<LitStr>,
        prefix: Option<&LitStr>,
    ) -> MacroResult<LitStr> {
        if let Some(uri) = explicit {
            return Ok(uri);
        }
        let Some(prefix) = prefix else {
            return Err(ProblemDiagnostic::MissingTypeUri {
                is_struct: self.is_struct,
            }
            .at(self.name));
        };
        let suffix = self.name.unraw().to_string().to_kebab_case();
        if suffix.is_empty() {
            return Err(ProblemDiagnostic::EmptyTypeSuffix.at(self.name));
        }
        let prefix = prefix.value();
        let separator = if prefix.ends_with([':', '/']) {
            ""
        } else {
            ":"
        };
        Ok(LitStr::new(
            &format!("{prefix}{separator}{suffix}"),
            self.name.span(),
        ))
    }

    fn resolve_title(&self, explicit: Option<LitStr>) -> LitStr {
        explicit.unwrap_or_else(|| {
            let title = self.name.unraw().to_string().to_title_case();
            LitStr::new(&title, self.name.span())
        })
    }

    fn check_identity(&self, uri: &LitStr, title: &LitStr) -> MacroResult<()> {
        if uri.value().is_empty() {
            return Err(ProblemDiagnostic::EmptyTypeUri.at(uri));
        }
        if title.value().is_empty() {
            return Err(ProblemDiagnostic::EmptyTitle.at(title));
        }
        Ok(())
    }

    fn collect(input: &'a DeriveInput) -> MacroResult<Vec<Self>> {
        match &input.data {
            Data::Enum(data) => Ok(data
                .variants
                .iter()
                .map(|variant| DeclarationSyntax {
                    name: &variant.ident,
                    attributes: &variant.attrs,
                    fields: &variant.fields,
                    is_struct: false,
                })
                .collect()),
            Data::Struct(data) => Ok(vec![DeclarationSyntax {
                name: &input.ident,
                attributes: &input.attrs,
                fields: &data.fields,
                is_struct: true,
            }]),
            Data::Union(_) => Err(ProblemDiagnostic::UnsupportedShape.at(&input.ident)),
        }
    }

    fn check_unique_constant(
        &self,
        name: &Ident,
        seen: &mut BTreeMap<String, Span>,
    ) -> MacroResult<()> {
        let name = name.to_string();
        if let Some(first) = seen.get(&name) {
            return Err(
                ProblemDiagnostic::DuplicateConstant { name }.conflict(self.name.span(), *first)
            );
        }
        seen.insert(name, self.name.span());
        Ok(())
    }

    fn check_variant_collision(
        &self,
        input: &DeriveInput,
        constant_name: &Ident,
    ) -> MacroResult<()> {
        let Data::Enum(data) = &input.data else {
            return Ok(());
        };
        if let Some(variant) = data
            .variants
            .iter()
            .find(|variant| variant.ident.unraw() == *constant_name)
        {
            return Err(ProblemDiagnostic::VariantConstantCollision {
                name: constant_name.to_string(),
            }
            .conflict(self.name.span(), variant.ident.span()));
        }
        Ok(())
    }
}

/****************************************/
/* Attribute Grammar                    */
/****************************************/

mod kw {
    syn::custom_keyword!(type_uri);
    syn::custom_keyword!(status);
    syn::custom_keyword!(title);
    syn::custom_keyword!(detail);
    syn::custom_keyword!(prefix);
    syn::custom_keyword!(data);
    syn::custom_keyword!(transparent);
}

/// Explicit metadata before defaults, prefix expansion, and field resolution.
#[derive(Default)]
struct ProblemAttributes {
    type_uri: Option<LitStr>,
    status: Option<u16>,
    title: Option<LitStr>,
    detail: Option<LitStr>,
}

impl ProblemAttributes {
    /// Merge declaration attributes, accepting named options or a standalone status literal.
    fn read(attributes: &[Attribute]) -> MacroResult<Self> {
        let mut parsed = Self::default();
        for attribute in attributes.problem_attributes() {
            if Self::is_status_shorthand(attribute)? {
                if parsed.status.is_some() {
                    return Err(
                        ProblemDiagnostic::DuplicateAttribute { name: "status" }.at(attribute)
                    );
                }
                parsed.status = Some(Self::status_number(attribute.parse_args()?)?);
            } else {
                attribute.parse_problem_args(|input: ParseStream| parsed.parse_named(input))?;
            }
        }
        Ok(parsed)
    }

    fn parse_named(&mut self, input: ParseStream) -> MacroResult<()> {
        if input.is_empty() {
            return Err(ProblemDiagnostic::EmptyAttribute.at_span(input.span()));
        }
        while !input.is_empty() {
            if input.peek(kw::type_uri) {
                let keyword: kw::type_uri = input.parse()?;
                Self::read_string(input, &mut self.type_uri, keyword.span, "type_uri")?;
            } else if input.peek(kw::status) {
                let keyword: kw::status = input.parse()?;
                self.read_status(input, keyword.span)?;
            } else if input.peek(kw::title) {
                let keyword: kw::title = input.parse()?;
                Self::read_string(input, &mut self.title, keyword.span, "title")?;
            } else if input.peek(kw::detail) {
                let keyword: kw::detail = input.parse()?;
                Self::read_string(input, &mut self.detail, keyword.span, "detail")?;
            } else {
                let path: syn::Path = input.parse()?;
                let diagnostic = if path.get_ident().is_some() {
                    ProblemDiagnostic::UnknownDeclarationAttribute
                } else {
                    ProblemDiagnostic::InvalidAttributeName
                };
                return Err(diagnostic.at(path));
            }
            if !input.is_empty() {
                input.parse::<syn::Token![,]>()?;
            }
        }
        Ok(())
    }

    fn read_status(&mut self, input: ParseStream, span: Span) -> MacroResult<()> {
        if self.status.is_some() {
            return Err(ProblemDiagnostic::DuplicateAttribute { name: "status" }.at_span(span));
        }
        let error = |error: syn::Error| ProblemDiagnostic::InvalidStatus.at_span(error.span());
        input.parse::<syn::Token![=]>().map_err(error)?;
        let expression = input.parse::<Expr>().map_err(error)?;
        self.status = Some(Self::status_number(expression)?);
        Ok(())
    }

    fn read_string(
        input: ParseStream,
        value: &mut Option<LitStr>,
        span: Span,
        name: &'static str,
    ) -> MacroResult<()> {
        if value.is_some() {
            return Err(ProblemDiagnostic::DuplicateAttribute { name }.at_span(span));
        }
        input.parse::<syn::Token![=]>()?;
        *value = Some(input.parse()?);
        Ok(())
    }

    fn status_number(expression: Expr) -> MacroResult<u16> {
        let Expr::Lit(literal) = expression else {
            return Err(ProblemDiagnostic::InvalidStatus.at(expression));
        };
        let Lit::Int(integer) = literal.lit else {
            return Err(ProblemDiagnostic::InvalidStatus.at(literal));
        };
        let number = integer
            .base10_parse::<u16>()
            .map_err(|_| ProblemDiagnostic::InvalidStatus.at(&integer))?;
        if !(100..=999).contains(&number) {
            return Err(ProblemDiagnostic::InvalidStatus.at(integer));
        }
        Ok(number)
    }

    fn is_status_shorthand(attribute: &Attribute) -> MacroResult<bool> {
        let first = attribute
            .meta
            .require_list()?
            .tokens
            .clone()
            .into_iter()
            .next();
        Ok(first.is_some_and(|token| match token {
            TokenTree::Literal(_) => true,
            TokenTree::Punct(punct) => punct.as_char() == '-',
            _ => false,
        }))
    }
}

/// Select and validate local data fields, or reserve a parameter for forwarded data.
impl<'a> Declaration<'a> {
    fn parse_public_data(&self, next_parameter: &mut usize) -> MacroResult<PublicData<'a>> {
        let mut selected = Vec::new();
        let mut names = BTreeMap::new();
        for (index, field) in self.syntax.fields.iter().enumerate() {
            let Some(serialized_name) = field.public_data_name()? else {
                continue;
            };
            if field.is_diagnostic_source() {
                return Err(ProblemDiagnostic::DiagnosticSourceData.at(&serialized_name));
            }
            let name = serialized_name.value();
            if let Some(first) = names.get(&name) {
                return Err(ProblemDiagnostic::DuplicateDataName { name }
                    .conflict(serialized_name.span(), *first));
            }
            names.insert(name, serialized_name.span());
            let parameter = format_ident!("F{}", *next_parameter);
            *next_parameter += 1;
            selected.push(DataField {
                selection: SelectedField::new(index, field),
                serialized_name,
                parameter,
            });
        }
        match self.kind {
            DeclarationKind::Transparent(ty) => {
                if let Some(field) = selected.first() {
                    return Err(ProblemDiagnostic::TransparentData.at(&field.serialized_name));
                }
                let parameter = format_ident!("F{}", *next_parameter);
                *next_parameter += 1;
                Ok(PublicData::Transparent { ty, parameter })
            }
            DeclarationKind::Defined(_) if selected.is_empty() => Ok(PublicData::None),
            DeclarationKind::Defined(_) => Ok(PublicData::Fields(selected)),
        }
    }
}

/****************************************/
/* Detail Formatting                    */
/****************************************/

impl<'a> DetailFormat<'a> {
    fn parse(text: &LitStr, fields: &'a Fields) -> MacroResult<Self> {
        let format = ParsedFormat::parse(text, matches!(fields, Fields::Unnamed(_)))?;
        let mut arguments = Vec::new();
        for (reference, format_traits) in format.references {
            let selection = reference.resolve(fields, text.span())?;
            let format_traits = format_traits
                .into_iter()
                .map(|name| Ident::new(name, text.span()))
                .collect();
            arguments.push(DetailArgument {
                selection,
                format_traits,
            });
        }
        Ok(Self {
            format_string: format.format_string,
            arguments,
        })
    }

    /// Require only the formatting traits used by the selected fields.
    fn formatting_bounds(&self) -> impl Iterator<Item = syn::WherePredicate> {
        self.arguments.iter().flat_map(|argument| {
            let ty = argument.selection.ty;
            argument
                .format_traits
                .iter()
                .map(move |format_trait| syn::parse_quote!(#ty: ::std::fmt::#format_trait))
        })
    }

    fn match_arm(&self, syntax: &DeclarationSyntax<'_>) -> Tokens {
        let selected = self
            .arguments
            .iter()
            .map(|argument| argument.selection.pattern_binding());
        let pattern = syntax.pattern(selected);
        let format_string = &self.format_string;
        let bindings = self
            .arguments
            .iter()
            .map(|argument| &argument.selection.binding);
        quote!(#pattern => ::std::option::Option::Some(::std::format!(#format_string, #(#bindings = #bindings),*)))
    }
}

/// Format syntax before field resolution. Repeated references share one argument.
///
/// Tuple placeholders are rewritten to named bindings so generated patterns and
/// format arguments use the same names. Rust validates the remaining format syntax.
struct ParsedFormat {
    format_string: LitStr,
    references: BTreeMap<DetailReference, BTreeSet<&'static str>>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum DetailReference {
    Index(usize),
    Name(String),
}

impl ParsedFormat {
    fn parse(text: &LitStr, tuple: bool) -> MacroResult<Self> {
        let mut references = BTreeMap::<DetailReference, BTreeSet<&'static str>>::new();
        let mut rewritten = String::new();
        let original = text.value();
        let mut chars = original.chars().peekable();

        while let Some(character) = chars.next() {
            match character {
                '{' | '}' if chars.peek() == Some(&character) => {
                    chars.next();
                    rewritten.push(character);
                    rewritten.push(character);
                }
                '{' => {
                    let placeholder = Self::read_placeholder(&mut chars, text.span())?;
                    let (name, specifier) =
                        placeholder.split_once(':').unwrap_or((&placeholder, ""));
                    let reference = DetailReference::parse(name, tuple, text.span())?;
                    Self::validate_format_parameters(specifier, text.span())?;
                    rewritten.push('{');
                    rewritten.push_str(&reference.argument_name());
                    if placeholder.contains(':') {
                        rewritten.push(':');
                        rewritten.push_str(specifier);
                    }
                    rewritten.push('}');
                    references
                        .entry(reference)
                        .or_default()
                        .insert(Self::formatting_trait(specifier));
                }
                '}' => {
                    return Err(ProblemDiagnostic::UnmatchedDetailBrace.at_span(text.span()));
                }
                _ => rewritten.push(character),
            }
        }
        Ok(Self {
            format_string: LitStr::new(&rewritten, text.span()),
            references,
        })
    }

    fn read_placeholder(
        chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
        span: Span,
    ) -> MacroResult<String> {
        let mut placeholder = String::new();
        loop {
            match chars.next() {
                Some('}') => return Ok(placeholder),
                Some('{') | None => {
                    return Err(ProblemDiagnostic::InvalidDetailFormat.at_span(span));
                }
                Some(character) => placeholder.push(character),
            }
        }
    }

    /// Reject dynamic arguments without mistaking an alignment fill character for one.
    fn validate_format_parameters(specifier: &str, span: Span) -> MacroResult<()> {
        let mut parameters = specifier.chars();
        let mut prefix = parameters.clone();
        if prefix.next().is_some() && prefix.next().is_some_and(|c| matches!(c, '<' | '^' | '>')) {
            parameters.next();
            parameters.next();
        }
        let parameters = parameters.as_str();
        if parameters.contains('$') || parameters.contains(".*") {
            return Err(ProblemDiagnostic::DynamicDetailFormat.at_span(span));
        }
        Ok(())
    }

    fn formatting_trait(specifier: &str) -> &'static str {
        match specifier.chars().last() {
            Some('?') => "Debug",
            Some('x') => "LowerHex",
            Some('X') => "UpperHex",
            Some('o') => "Octal",
            Some('b') => "Binary",
            Some('p') => "Pointer",
            Some('e') => "LowerExp",
            Some('E') => "UpperExp",
            _ => "Display",
        }
    }
}

impl DetailReference {
    fn parse(name: &str, tuple: bool, span: Span) -> MacroResult<Self> {
        if tuple {
            if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(ProblemDiagnostic::TupleDetailIndexRequired.at_span(span));
            }
            let index = name
                .parse()
                .map_err(|_| ProblemDiagnostic::TupleDetailIndexOutOfRange.at_span(span))?;
            Ok(Self::Index(index))
        } else {
            Ident::parse_any
                .parse_str(name)
                .map_err(|_| ProblemDiagnostic::NamedDetailFieldRequired.at_span(span))?;
            Ok(Self::Name(name.to_owned()))
        }
    }

    fn argument_name(&self) -> String {
        match self {
            Self::Index(index) => format!("field_{index}"),
            Self::Name(name) => name.clone(),
        }
    }

    /// Resolve an original field and reject diagnostic sources.
    ///
    /// Unknown-field notes list eligible names or original tuple indexes.
    fn resolve<'a>(&self, fields: &'a Fields, span: Span) -> MacroResult<SelectedField<'a>> {
        let selected = match self {
            Self::Index(index) => fields.iter().nth(*index).map(|field| (*index, field)),
            Self::Name(name) => fields.iter().enumerate().find(|(_, field)| {
                field
                    .ident
                    .as_ref()
                    .is_some_and(|ident| ident.unraw() == name.as_str())
            }),
        };
        let (index, field) = selected.ok_or_else(|| {
            let name = match self {
                Self::Index(index) => index.to_string(),
                Self::Name(name) => name.clone(),
            };
            let names = fields
                .iter()
                .enumerate()
                .filter(|(_, field)| !field.is_diagnostic_source())
                .map(|(index, field)| {
                    field
                        .ident
                        .as_ref()
                        .map_or_else(|| index.to_string(), |name| name.unraw().to_string())
                })
                .collect::<Vec<_>>()
                .join(", ");
            let available = if names.is_empty() {
                "this declaration has no fields that can appear in public detail".to_owned()
            } else {
                format!("available detail fields: {names}")
            };
            ProblemDiagnostic::UnknownDetailField { name, available }.at_span(span)
        })?;
        if field.is_diagnostic_source() {
            return Err(ProblemDiagnostic::DiagnosticSourceDetail.at_span(span));
        }
        Ok(SelectedField::new(index, field))
    }
}

/****************************************/
/* Problem Implementation               */
/****************************************/

impl ProblemDerive<'_> {
    /// Emit the payload container, definition constants, and runtime trait implementation.
    fn generate(&self, runtime: &Tokens) -> Tokens {
        let data = self.data_container(runtime);
        let constants = self.definition_constants(runtime);
        let implementation = self.problem_implementation(runtime);
        quote!(#data #constants #implementation)
    }

    fn definition_constants(&self, runtime: &Tokens) -> Tokens {
        let name = &self.input.ident;
        let (generics, ty, bounds) = self.input.generics.split_for_impl();
        let constants = self.declarations.iter().filter_map(|declaration| {
            let DeclarationKind::Defined(metadata) = &declaration.kind else {
                return None;
            };
            let constant_name = &metadata.constant_name;
            let value = metadata.generate(runtime);
            Some(quote! {
                pub const #constant_name: #runtime::ProblemDefinition = #value;
            })
        });
        quote!(impl #generics #name #ty #bounds { #(#constants)* })
    }

    /// Preserve input generics and add the bounds required by public data and detail.
    fn problem_implementation(&self, runtime: &Tokens) -> Tokens {
        let name = &self.input.ident;
        let mut generics = self.input.generics.clone();
        let bounds = generics.make_where_clause();
        bounds
            .predicates
            .push(syn::parse_quote!(Self: ::std::error::Error));
        for declaration in &self.declarations {
            bounds.predicates.extend(declaration.bounds(runtime));
        }
        let (generics, ty, bounds) = generics.split_for_impl();
        let owned = self.data_type(runtime, DataOwnership::Owned);
        let borrowed = self.data_type(runtime, DataOwnership::Borrowed);
        let data_name = self.data_name();
        let data = self
            .declarations
            .iter()
            .map(|declaration| declaration.data_arm(runtime, &data_name, DataOwnership::Borrowed));
        let into_data = self
            .declarations
            .iter()
            .map(|declaration| declaration.data_arm(runtime, &data_name, DataOwnership::Owned));
        let definition = self
            .declarations
            .iter()
            .map(|declaration| declaration.definition_arm(runtime));
        let detail = self
            .declarations
            .iter()
            .map(|declaration| declaration.detail_arm(runtime));
        let instance = self
            .declarations
            .iter()
            .map(|declaration| declaration.instance_arm(runtime));
        let definitions = self.definition_iterator(runtime);
        // --- An empty enum needs a value match to prove it has no inhabitants.
        let scrutinee = if self.declarations.is_empty() {
            quote!(*self)
        } else {
            quote!(self)
        };
        quote! {
            impl #generics #runtime::Problem for #name #ty #bounds {
                type Data = #owned;
                type DataRef<'__problem_data> = #borrowed where Self: '__problem_data;
                fn data(&self) -> Option<Self::DataRef<'_>> {
                    match #scrutinee { #(#data),* }
                }
                fn into_data(self) -> Option<Self::Data> {
                    match self { #(#into_data),* }
                }
                fn definition(&self) -> &'static #runtime::ProblemDefinition {
                    match #scrutinee { #(#definition),* }
                }
                fn definitions() -> impl ::std::iter::Iterator<Item = &'static #runtime::ProblemDefinition> {
                    #definitions
                }
                fn detail(&self) -> ::std::option::Option<::std::string::String> {
                    match #scrutinee { #(#detail),* }
                }
                fn instance(&self) -> ::std::option::Option<::std::string::String> {
                    match #scrutinee { #(#instance),* }
                }
            }
        }
    }

    /// Chain local and forwarded definitions in the original declaration order.
    fn definition_iterator(&self, runtime: &Tokens) -> Tokens {
        let empty = quote!(::std::iter::empty::<&'static #runtime::ProblemDefinition>());
        self.declarations
            .iter()
            .fold(empty, |previous, declaration| {
                let next = match &declaration.kind {
                    DeclarationKind::Defined(metadata) => {
                        let constant_name = &metadata.constant_name;
                        quote!(::std::iter::once(&Self::#constant_name))
                    }
                    DeclarationKind::Transparent(ty) => {
                        quote!(<#ty as #runtime::Problem>::definitions())
                    }
                };
                quote!(::std::iter::Iterator::chain(#previous, #next))
            })
    }
}

impl ProblemMetadata<'_> {
    fn generate(&self, runtime: &Tokens) -> Tokens {
        let type_uri = &self.type_uri;
        let title = &self.title;
        let number = self.status;
        quote!(#runtime::ProblemDefinition {
            type_uri: #type_uri,
            title: #title,
            status: const {
                match #runtime::StatusCode::from_u16(#number) {
                    ::std::result::Result::Ok(status) => status,
                    ::std::result::Result::Err(_) => panic!("status validated by Problem derive"),
                }
            },
        })
    }

    fn check_unique_uri(uri: &LitStr, seen: &mut BTreeMap<String, Span>) -> MacroResult<()> {
        let value = uri.value();
        if let Some(first) = seen.get(&value) {
            return Err(
                ProblemDiagnostic::DuplicateTypeUri { uri: value }.conflict(uri.span(), *first)
            );
        }
        seen.insert(value, uri.span());
        Ok(())
    }
}

impl Declaration<'_> {
    fn bounds(&self, runtime: &Tokens) -> Vec<syn::WherePredicate> {
        let mut bounds = match &self.kind {
            DeclarationKind::Transparent(ty) => vec![syn::parse_quote!(#ty: #runtime::Problem)],
            DeclarationKind::Defined(metadata) => metadata
                .detail
                .iter()
                .flat_map(DetailFormat::formatting_bounds)
                .collect(),
        };
        if let PublicData::Fields(fields) = &self.public_data {
            for field in fields {
                let ty = field.selection.ty;
                bounds.push(syn::parse_quote!(#ty: #runtime::__private::serde::Serialize));
            }
        }
        bounds
    }

    fn definition_arm(&self, runtime: &Tokens) -> Tokens {
        match &self.kind {
            DeclarationKind::Defined(metadata) => {
                let pattern = self.syntax.pattern([]);
                let constant_name = &metadata.constant_name;
                quote!(#pattern => &Self::#constant_name)
            }
            DeclarationKind::Transparent(_) => {
                let constructor = self.syntax.constructor();
                quote!(#constructor(problem) => #runtime::Problem::definition(problem))
            }
        }
    }

    fn detail_arm(&self, runtime: &Tokens) -> Tokens {
        match &self.kind {
            DeclarationKind::Transparent(_) => {
                let constructor = self.syntax.constructor();
                quote!(#constructor(problem) => #runtime::Problem::detail(problem))
            }
            DeclarationKind::Defined(metadata) => {
                if let Some(detail) = &metadata.detail {
                    return detail.match_arm(&self.syntax);
                }
                let pattern = self.syntax.pattern([]);
                quote!(#pattern => ::std::option::Option::None)
            }
        }
    }

    fn instance_arm(&self, runtime: &Tokens) -> Tokens {
        match self.kind {
            DeclarationKind::Defined(_) => {
                let pattern = self.syntax.pattern([]);
                quote!(#pattern => ::std::option::Option::None)
            }
            DeclarationKind::Transparent(_) => {
                let constructor = self.syntax.constructor();
                quote!(#constructor(problem) => #runtime::Problem::instance(problem))
            }
        }
    }
}

impl DeclarationSyntax<'_> {
    fn constructor(&self) -> Tokens {
        let name = self.name;
        if self.is_struct {
            quote!(Self)
        } else {
            quote!(Self::#name)
        }
    }

    /// Bind selected fields and ignore the rest, preserving original tuple positions.
    fn pattern(&self, selected: impl IntoIterator<Item = (usize, Ident)>) -> Tokens {
        let constructor = self.constructor();
        let selected: BTreeMap<_, _> = selected.into_iter().collect();
        match self.fields {
            Fields::Unit => quote!(#constructor),
            Fields::Named(_) => {
                let bindings = selected.values();
                quote!(#constructor { #(#bindings,)* .. })
            }
            Fields::Unnamed(_) if selected.is_empty() => quote!(#constructor(..)),
            Fields::Unnamed(_) => {
                let bindings = (0..self.fields.len()).map(|index| {
                    selected
                        .get(&index)
                        .map_or_else(|| quote!(_), |name| quote!(#name))
                });
                quote!(#constructor(#(#bindings),*))
            }
        }
    }
}

/****************************************/
/* Public Data Generation               */
/****************************************/

/// Choose references for `data` or moved field values for `into_data`.
#[derive(Clone, Copy)]
enum DataOwnership {
    Borrowed,
    Owned,
}

impl ProblemDerive<'_> {
    fn data_name(&self) -> Ident {
        format_ident!("{}Data", self.input.ident.unraw())
    }

    /// Fill container parameters with borrowed or owned types; use `()` without data.
    fn data_type(&self, runtime: &Tokens, ownership: DataOwnership) -> Tokens {
        let types: Vec<_> = self
            .declarations
            .iter()
            .flat_map(|declaration| declaration.public_data.types(runtime, ownership))
            .collect();
        if types.is_empty() {
            return quote!(());
        }
        let name = self.data_name();
        quote!(#name<#(#types),*>)
    }

    /// Generate one container shared by borrowed and owned payloads.
    ///
    /// Generic field parameters avoid duplicating the shape for references and values.
    /// Only declarations with payloads appear in the generated untagged data enum.
    fn data_container(&self, runtime: &Tokens) -> Tokens {
        let parameters: Vec<_> = self
            .declarations
            .iter()
            .flat_map(|declaration| declaration.public_data.parameters())
            .collect();
        if parameters.is_empty() {
            return quote!();
        }
        let name = self.data_name();
        let visibility = &self.input.vis;
        let shapes: Vec<_> = self
            .declarations
            .iter()
            .filter_map(Declaration::data_shape)
            .collect();
        let body = match self.input.data {
            Data::Struct(_) => {
                let fields = &shapes[0];
                quote!(#visibility struct #name<#(#parameters),*> #fields)
            }
            _ => {
                quote!(#[serde(untagged)] #visibility enum #name<#(#parameters),*> { #(#shapes),* })
            }
        };
        // --- Route derives through runtime re-exports, including renamed dependencies.
        let serde_path = quote!(#runtime::__private::serde)
            .to_string()
            .replace(' ', "");
        let schema_path = quote!(#runtime::__private::schemars)
            .to_string()
            .replace(' ', "");
        quote! {
            #runtime::__problem_data! {
                #schema_path,
                #[derive(Debug, Clone, PartialEq, Eq, #runtime::__private::serde::Serialize, #runtime::__private::serde::Deserialize)]
                #[serde(crate = #serde_path)]
                #body
            }
        }
    }
}

impl PublicData<'_> {
    fn parameters(&self) -> Vec<&Ident> {
        match self {
            Self::None => Vec::new(),
            Self::Fields(fields) => fields.iter().map(|field| &field.parameter).collect(),
            Self::Transparent { parameter, .. } => vec![parameter],
        }
    }

    fn types(&self, runtime: &Tokens, ownership: DataOwnership) -> Vec<Tokens> {
        match self {
            Self::None => Vec::new(),
            Self::Fields(fields) => fields
                .iter()
                .map(|field| {
                    let ty = field.selection.ty;
                    match ownership {
                        DataOwnership::Borrowed => quote!(&'__problem_data #ty),
                        DataOwnership::Owned => quote!(#ty),
                    }
                })
                .collect(),
            Self::Transparent { ty, .. } => vec![match ownership {
                DataOwnership::Borrowed => {
                    quote!(<#ty as #runtime::Problem>::DataRef<'__problem_data>)
                }
                DataOwnership::Owned => quote!(<#ty as #runtime::Problem>::Data),
            }],
        }
    }
}

impl Declaration<'_> {
    /// Describe selected fields or a forwarded payload; omit declarations without data.
    fn data_shape(&self) -> Option<Tokens> {
        let variant = self.syntax.name;
        match &self.public_data {
            PublicData::None => None,
            PublicData::Transparent { parameter, .. } => Some(quote!(#variant(#parameter))),
            PublicData::Fields(fields) => {
                let visibility = if self.syntax.is_struct {
                    quote!(pub)
                } else {
                    quote!()
                };
                let members = fields.iter().map(|field| {
                    let name = &field.selection.binding;
                    let serialized_name = &field.serialized_name;
                    let parameter = &field.parameter;
                    quote!(#[serde(rename = #serialized_name)] #visibility #name: #parameter)
                });
                Some(if self.syntax.is_struct {
                    quote!({ #(#members),* })
                } else {
                    quote!(#variant { #(#members),* })
                })
            }
        }
    }

    fn data_arm(&self, runtime: &Tokens, name: &Ident, ownership: DataOwnership) -> Tokens {
        let variant = self.syntax.name;
        let public_data = if self.syntax.is_struct {
            quote!(#name)
        } else {
            quote!(#name::#variant)
        };
        match &self.public_data {
            PublicData::None => {
                let pattern = self.syntax.pattern([]);
                quote!(#pattern => None)
            }
            PublicData::Transparent { .. } => {
                let constructor = self.syntax.constructor();
                let method = match ownership {
                    DataOwnership::Borrowed => quote!(data),
                    DataOwnership::Owned => quote!(into_data),
                };
                quote!(#constructor(problem) => #runtime::Problem::#method(problem).map(#public_data))
            }
            PublicData::Fields(fields) => {
                let selected = fields.iter().map(|field| field.selection.pattern_binding());
                let pattern = self.syntax.pattern(selected);
                let bindings = fields.iter().map(|field| &field.selection.binding);
                quote!(#pattern => Some(#public_data { #(#bindings),* }))
            }
        }
    }
}

/****************************************/
/* Syntax Extensions                    */
/****************************************/

/// Bridge Syn attribute parsing to the macro's compiler-error representation.
trait ProblemAttributeExt {
    fn parse_problem_args<T>(
        &self,
        parser: impl FnOnce(ParseStream) -> MacroResult<T>,
    ) -> MacroResult<T>;
}

impl ProblemAttributeExt for Attribute {
    fn parse_problem_args<T>(
        &self,
        parser: impl FnOnce(ParseStream) -> MacroResult<T>,
    ) -> MacroResult<T> {
        // --- Syn requires its own error type; retain the richer error outside the parser.
        let mut diagnostic = None;
        let parsed = self.parse_args_with(|input: ParseStream| {
            parser(input).map_err(|error| {
                diagnostic = Some(error);
                input.error("invalid problem attribute")
            })
        });
        // --- Restore domain diagnostics, or preserve native Syn errors for malformed syntax.
        parsed.map_err(|error| diagnostic.unwrap_or_else(|| error.into()))
    }
}

/// Read enum-level options and detect exclusive transparent forwarding.
trait ProblemAttributesExt {
    fn problem_attributes(&self) -> impl Iterator<Item = &Attribute>;
    fn parse_prefix(&self) -> MacroResult<Option<LitStr>>;
    fn is_transparent(&self) -> MacroResult<bool>;
}

impl ProblemAttributesExt for [Attribute] {
    fn problem_attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.iter()
            .filter(|attribute| attribute.path().is_ident("problem"))
    }

    fn parse_prefix(&self) -> MacroResult<Option<LitStr>> {
        let mut prefix = None;
        for attribute in self.problem_attributes() {
            attribute.parse_problem_args(|input: ParseStream| {
                if input.is_empty() {
                    return Err(ProblemDiagnostic::EmptyAttribute.at_span(input.span()));
                }
                while !input.is_empty() {
                    if !input.peek(kw::prefix) {
                        let path: syn::Path = input.parse()?;
                        return Err(ProblemDiagnostic::UnknownEnumAttribute.at(path));
                    }
                    let keyword: kw::prefix = input.parse()?;
                    if prefix.is_some() {
                        return Err(
                            ProblemDiagnostic::DuplicateAttribute { name: "prefix" }.at(keyword)
                        );
                    }
                    input.parse::<syn::Token![=]>()?;
                    let value: LitStr = input.parse()?;
                    if value.value().is_empty() {
                        return Err(ProblemDiagnostic::EmptyPrefix.at(&value));
                    }
                    prefix = Some(value);
                    if !input.is_empty() {
                        input.parse::<syn::Token![,]>()?;
                    }
                }
                Ok(())
            })?;
        }
        Ok(prefix)
    }

    fn is_transparent(&self) -> MacroResult<bool> {
        let attributes: Vec<_> = self.problem_attributes().collect();
        for attribute in &attributes {
            let Ok(items) = attribute.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            ) else {
                continue;
            };
            if items.iter().any(|item| item.path().is_ident("transparent")) {
                if attributes.len() != 1
                    || items.len() != 1
                    || !matches!(items.first(), Some(syn::Meta::Path(_)))
                {
                    return Err(ProblemDiagnostic::TransparentAttributes.at(attribute));
                }
                attribute.parse_problem_args(|input: ParseStream| {
                    input.parse::<kw::transparent>()?;
                    if input.peek(syn::Token![,]) {
                        input.parse::<syn::Token![,]>()?;
                    }
                    Ok(())
                })?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Select public fields and recognize diagnostic sources from names and attributes.
trait ProblemFieldExt {
    fn public_data_name(&self) -> MacroResult<Option<LitStr>>;
    fn is_diagnostic_source(&self) -> bool;
}

impl ProblemFieldExt for Field {
    fn public_data_name(&self) -> MacroResult<Option<LitStr>> {
        let mut name = None;
        for attribute in self.attrs.problem_attributes() {
            attribute.parse_problem_args(|input: ParseStream| {
                if input.is_empty() {
                    return Err(ProblemDiagnostic::EmptyAttribute.at_span(input.span()));
                }
                while !input.is_empty() {
                    if !input.peek(kw::data) {
                        let path: syn::Path = input.parse()?;
                        return Err(ProblemDiagnostic::UnknownFieldAttribute.at(path));
                    }
                    let keyword: kw::data = input.parse()?;
                    if name.is_some() {
                        return Err(
                            ProblemDiagnostic::DuplicateAttribute { name: "data" }.at(keyword)
                        );
                    }
                    let value = if input.peek(syn::Token![=]) {
                        input.parse::<syn::Token![=]>()?;
                        input.parse::<LitStr>()?
                    } else {
                        let ident = self.ident.as_ref().ok_or_else(|| {
                            ProblemDiagnostic::TupleDataNameRequired.at_span(keyword.span)
                        })?;
                        LitStr::new(&ident.unraw().to_string(), ident.span())
                    };
                    if value.value().is_empty() {
                        return Err(ProblemDiagnostic::EmptyDataName.at(&value));
                    }
                    name = Some(value);
                    if !input.is_empty() {
                        input.parse::<syn::Token![,]>()?;
                    }
                }
                Ok(())
            })?;
        }
        Ok(name)
    }

    fn is_diagnostic_source(&self) -> bool {
        fn has_source_token(tokens: Tokens) -> bool {
            tokens.into_iter().any(|token| match token {
                TokenTree::Ident(ident) => ident == "source",
                TokenTree::Group(group) => has_source_token(group.stream()),
                _ => false,
            })
        }

        self.ident
            .as_ref()
            .is_some_and(|name| name.unraw() == "source")
            || self.attrs.iter().any(|attribute| {
                attribute.path().is_ident("source")
                    || attribute.path().is_ident("from")
                    || (attribute.path().is_ident("error")
                        && has_source_token(attribute.meta.to_token_stream()))
            })
    }
}

/****************************************/
/* Runtime Crate Resolution             */
/****************************************/

/// Find the runtime path in the consuming crate, respecting dependency renames.
fn resolve_runtime_crate() -> MacroResult<Tokens> {
    let name = proc_macro_crate::crate_name("problems")
        .map_err(|error| syn::Error::new(Span::call_site(), error))?;
    Ok(match name {
        proc_macro_crate::FoundCrate::Itself => quote!(::problems),
        proc_macro_crate::FoundCrate::Name(name) => {
            let name = Ident::new(&name, Span::call_site());
            quote!(::#name)
        }
    })
}
