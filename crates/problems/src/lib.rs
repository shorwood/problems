//! Declares public HTTP problems and projects them into RFC 9457 documents.
//!
//! Errors retain their diagnostic formatting and source chains. Reports expose
//! the declared public metadata and can be serialized with any Serde encoder.
//! The default `derive` feature supports enum declarations. Optional `axum`,
//! `actix-web`, `rocket`, `poem`, `salvo`, and `warp` features provide JSON HTTP
//! responses. `schemars` provides document schemas; `aide` adds declared OpenAPI
//! response statuses.

extern crate self as problems;

// Re-export dependencies used by generated problem data containers.
pub use http::StatusCode;

// Re-export the derive macro when enabled.
#[cfg(feature = "derive")]
pub use problems_derive::Problem;

/****************************************/
/* Macro Support                        */
/****************************************/

/// Support used by generated problem data containers.
#[doc(hidden)]
pub mod __private {
    #[cfg(feature = "schemars")]
    pub use schemars;
    pub use serde;

    #[doc(hidden)]
    #[cfg(feature = "schemars")]
    #[macro_export]
    macro_rules! __problem_data {
        ($schema_crate:literal, $item:item) => {
            #[derive($crate::__private::schemars::JsonSchema)]
            #[schemars(crate = $schema_crate)]
            $item
        };
    }

    #[doc(hidden)]
    #[cfg(not(feature = "schemars"))]
    #[macro_export]
    macro_rules! __problem_data {
        ($schema_crate:literal, $item:item) => {
            $item
        };
    }
}

/****************************************/
/* Problem Definition                   */
/****************************************/

/// Defines the identity, title, and HTTP status shared by occurrences of a problem.
/// Reports use this metadata for their public document. OpenAPI generation uses
/// the same definitions to describe the responses an operation can return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProblemDefinition {
    /// RFC 9457 `type`, used by clients as the primary problem identifier.
    /// The caller supplies a valid URI reference; syntax is not validated.
    pub type_uri: &'static str,
    /// Short summary, stable between occurrences.
    pub title: &'static str,
    /// HTTP response status, also included in the public document.
    pub status: StatusCode,
}

/****************************************/
/* Problem                               */
/****************************************/

/// Attaches public problem metadata to an ordinary Rust error.
/// Diagnostic formatting and source chains remain the responsibility of the error.
/// Only the definition, detail, instance, and explicitly selected data enter the public document.
///
/// Implement this trait directly, or use `#[derive(Problem)]` with the `derive`
/// feature, enabled by default. Unit, named-field, and tuple variants or structs
/// are accepted. Struct metadata goes on the type, requires `type_uri`, and
/// exposes `DEFINITION`; `definitions()` yields that single definition.
/// Tuple detail uses explicit indexes such as `{0}`; diagnostic sources cannot
/// be interpolated.
/// The derive implements `definition()` and
/// `definitions()` from each variant's `#[problem(...)]` declaration. It also
/// implements `detail()` when a detail format is declared, using the variant's
/// named fields or explicit tuple indexes. Status accepts an integer literal from 100 through 999 and defaults to
/// `StatusCode::INTERNAL_SERVER_ERROR` when omitted. `instance()` defaults to `None`.
/// `#[problem(409)]` abbreviates `#[problem(status = 409)]`. Titles default to
/// the variant name in Title Case, such as `NameConflict` becoming `Name Conflict`.
/// An enum prefix generates type URIs from kebab-case variant names. Explicit
/// variant `type_uri` declarations override it; without a prefix they are required.
/// Each locally declared variant exposes a definition constant in SHOUTY_SNAKE_CASE,
/// such as `NAME_CONFLICT`. Transparent variants use their wrapped definitions.
/// Prefixes ending in `:` or `/` are used directly; otherwise a colon is appended.
/// Generated URIs change when variants are renamed. Keep an explicit URI when
/// renaming a variant must preserve its public identity. Duplicate locally declared
/// URIs are rejected; delegated lists are chained as provided by their types.
/// Single-field tuple variants marked `#[problem(transparent)]` delegate their
/// definition, detail, and instance to the wrapped `Problem`. Their definitions
/// are chained into the enclosing enum's iterator in variant order.
///
/// Select named fields with `#[problem(data)]`, or rename with
/// `#[problem(data = "public_name")]`. Tuple fields require explicit names.
/// Sources cannot be selected. The generated `<Type>Data` container is generic
/// over selected fields; borrowed projections use references, consuming
/// projections move fields.
///
/// The derive leaves `Display` and `Error` to your error implementation.
/// For example, declare a conflict and include its name in the public explanation:
///
/// ```rust
/// # #[cfg(feature = "derive")]
/// # {
/// use problems::{IntoReport, Problem, StatusCode};
///
/// #[derive(Debug, thiserror::Error, problems::Problem)]
/// #[problem(prefix = "urn:example")]
/// enum CreateProblem {
///     #[error("duplicate name: {name}")]
///     #[problem(
///         status = 409,
///         title = "Name conflict",
///         detail = "The name '{name}' is already in use."
///     )]
///     NameConflict { name: String },
/// }
///
/// let report = CreateProblem::NameConflict { name: "example".into() }.into_report();
/// assert_eq!(report.problem().definition().status, StatusCode::CONFLICT);
/// assert_eq!(report.problem().definition().type_uri, "urn:example:name-conflict");
/// assert_eq!(report.problem().detail().as_deref(), Some("The name 'example' is already in use."));
/// # }
/// ```
pub trait Problem: std::error::Error {
    /// Owned fields explicitly selected for the public document.
    type Data: serde::Serialize
    where
        Self: Sized;

    /// Borrowed projection of selected public fields.
    type DataRef<'a>: serde::Serialize
    where
        Self: Sized + 'a;

    /// Borrow selected public fields without cloning diagnostic state.
    fn data(&self) -> Option<Self::DataRef<'_>>
    where
        Self: Sized,
    {
        None
    }

    /// Move selected public fields into an owned document.
    fn into_data(self) -> Option<Self::Data>
    where
        Self: Sized,
    {
        None
    }

    /// Metadata for this occurrence's problem type.
    fn definition(&self) -> &'static ProblemDefinition;

    /// Lists all definitions exposed by this error type without constructing values.
    /// Every definition returned by `definition()` must appear in this list so
    /// OpenAPI generation describes every possible response.
    /// Returns an iterator of static references so composed problems can chain
    /// definition lists without allocating or constructing error values.
    fn definitions() -> impl Iterator<Item = &'static ProblemDefinition>
    where
        Self: Sized;

    /// Optional public explanation for this occurrence.
    fn detail(&self) -> Option<String> {
        None
    }

    /// Optional URI reference identifying this occurrence.
    fn instance(&self) -> Option<String> {
        None
    }
}

/****************************************/
/* Problem Details                      */
/****************************************/

/// Details of an HTTP API error, following RFC 9457.
///
/// Identifies the problem type and describes the individual occurrence.
/// Optional members are omitted when unavailable. Equality compares all
/// public members, including the occurrence URI.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ProblemDetails<D = ()> {
    /// URI reference identifying the problem type.
    ///
    /// Use this value as the primary identifier when handling errors.
    /// `about:blank` indicates that the HTTP status supplies the problem's meaning.
    #[serde(rename = "type")]
    type_uri: &'static str,

    /// Human-readable summary of the problem type.
    ///
    /// Remains stable across occurrences of the same problem type, except for
    /// localization. Use `type` to identify the problem programmatically.
    title: &'static str,

    /// HTTP status code generated by the origin server for this occurrence.
    ///
    /// This value is advisory; intermediaries may change the HTTP response status.
    #[serde(serialize_with = "serialize_status")]
    #[cfg_attr(
        feature = "schemars",
        schemars(with = "u16", range(min = 100, max = 999))
    )]
    status: StatusCode,

    /// Human-readable explanation specific to this occurrence.
    ///
    /// May help resolve the problem. Clients should not parse this text to
    /// extract information for programmatic handling. Omitted when unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,

    /// URI reference identifying this particular occurrence of the problem.
    ///
    /// May identify the occurrence without providing a retrievable resource.
    /// Omitted when unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    instance: Option<String>,

    /// Explicit public values for programmatic handling or interpolation.
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<D>,
}

fn serialize_status<S: serde::Serializer>(
    status: &StatusCode,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_u16(status.as_u16())
}

impl<D> ProblemDetails<D> {
    /// HTTP status represented by this body.
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// Stable problem identity.
    pub const fn type_uri(&self) -> &'static str {
        self.type_uri
    }

    /// Stable public title.
    pub const fn title(&self) -> &'static str {
        self.title
    }

    /// Borrow the public explanation for this occurrence.
    /// Returns `None` when the member is omitted from the document.
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// Borrow the URI reference identifying this occurrence.
    /// Returns `None` when the member is omitted from the document.
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_deref()
    }

    /// Borrow explicitly selected public values.
    pub fn data(&self) -> Option<&D> {
        self.data.as_ref()
    }
}

/****************************************/
/* Problem Document                     */
/****************************************/

/// Owned public problem metadata with a caller-selected payload type.
///
/// Missing type defaults to `about:blank`; other missing members are absent.
/// Unknown extensions are discarded. Decoding uses ordinary Serde type checks:
/// incorrectly typed members fail decoding rather than being ignored as RFC 9457
/// prescribes. Status values must be integers within 100–999.
///
/// Equality compares all public members, including the occurrence URI.
/// `matches()` compares with typed producer details while ignoring that URI.
///
/// This document retains neither diagnostic sources nor static definitions. It
/// does not implement `Problem` or framework response traits.
/// `D` selects the public payload type used by the deserializer. Use `()` for
/// problems without a payload or a format's value type for arbitrary data.
/// Missing or null data is absent.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ProblemDocument<D = ()> {
    /// URI reference identifying the received problem type.
    ///
    /// Defaults to `about:blank` when the member is absent.
    #[serde(rename = "type", default = "default_type")]
    type_uri: String,

    /// Human-readable summary supplied by the producer.
    ///
    /// Absent when the document does not supply a title.
    title: Option<String>,

    /// Advisory HTTP status code supplied by the producer.
    ///
    /// Absent when unavailable. The actual HTTP response status remains separate.
    #[serde(default, deserialize_with = "deserialize_status")]
    #[cfg_attr(
        feature = "schemars",
        schemars(with = "Option<u16>", range(min = 100, max = 999))
    )]
    status: Option<StatusCode>,

    /// Human-readable explanation specific to this occurrence.
    ///
    /// Absent when unavailable. Clients should not parse this text for
    /// programmatic handling.
    detail: Option<String>,

    /// URI reference identifying this particular occurrence.
    ///
    /// Absent when unavailable. Relative references remain unresolved.
    instance: Option<String>,

    /// Explicit public values decoded into the caller's payload type.
    data: Option<D>,
}

fn default_type() -> String {
    "about:blank".into()
}

fn deserialize_status<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<StatusCode>, D::Error> {
    <Option<u16> as serde::Deserialize<'de>>::deserialize(deserializer)?
        .map(StatusCode::from_u16)
        .transpose()
        .map_err(serde::de::Error::custom)
}

impl<D> ProblemDocument<D> {
    /// Compare the received problem identity with a declared definition.
    ///
    /// Compares only the type URI, not occurrence data or diagnostics. Relative
    /// received references must be resolved by the caller before comparison.
    ///
    /// ```rust
    /// # #[cfg(feature = "derive")]
    /// # {
    /// use problems::ProblemDocument;
    ///
    /// #[derive(Debug, thiserror::Error, problems::Problem)]
    /// #[problem(prefix = "urn:example")]
    /// enum CreateProblem {
    ///     #[error("private diagnostic: {source}")]
    ///     #[problem(409)]
    ///     NameConflict { source: std::io::Error },
    ///
    ///     #[error("resource missing")]
    ///     #[problem(404)]
    ///     Missing,
    /// }
    ///
    /// let received: ProblemDocument = serde_json::from_value(serde_json::json!({
    ///     "type": "urn:example:name-conflict"
    /// }))?;
    ///
    /// // Identify the variant without constructing its diagnostic source.
    /// assert!(received.is_type(&CreateProblem::NAME_CONFLICT));
    /// assert!(!received.is_type(&CreateProblem::MISSING));
    /// # }
    /// # Ok::<(), serde_json::Error>(())
    /// ```
    pub fn is_type(&self, definition: &ProblemDefinition) -> bool {
        self.type_uri == definition.type_uri
    }

    /// Compare public failure data with typed producer details, ignoring instance.
    ///
    /// Type URI, title, status, detail, and data must match exactly. Missing members
    /// are not wildcards. Convert details with `ProblemDocument::from()` and use
    /// `==` to compare the occurrence URI as well.
    ///
    /// ```rust
    /// # #[cfg(feature = "derive")]
    /// # {
    /// use problems::{ProblemDocument, IntoReport};
    ///
    /// #[derive(Debug, thiserror::Error, problems::Problem)]
    /// #[problem(prefix = "urn:example")]
    /// enum CreateProblem {
    ///     #[error("private diagnostic: {name}")]
    ///     #[problem(status = 409, detail = "The name '{name}' is already in use.")]
    ///     NameConflict { name: String },
    /// }
    ///
    /// let expected = CreateProblem::NameConflict {
    ///     name: "monthly".into(),
    /// }
    /// .into_report()
    /// .with_instance("/occurrences/1")
    /// .into_details();
    ///
    /// let received: ProblemDocument = serde_json::from_value(serde_json::json!({
    ///     "type": "urn:example:name-conflict",
    ///     "title": "Name Conflict",
    ///     "status": 409,
    ///     "detail": "The name 'monthly' is already in use.",
    ///     "instance": "/occurrences/2"
    /// }))?;
    ///
    /// // The same failure occurred twice, with different occurrence URIs.
    /// assert!(received.matches(&expected));
    /// assert_ne!(received, ProblemDocument::from(expected));
    ///
    /// let different_name = CreateProblem::NameConflict {
    ///     name: "weekly".into(),
    /// }
    /// .into_report()
    /// .into_details();
    ///
    /// // Detail participates in matching: "weekly" differs from "monthly".
    /// assert!(!received.matches(&different_name));
    /// # }
    /// # Ok::<(), serde_json::Error>(())
    /// ```
    pub fn matches<P>(&self, expected: &ProblemDetails<P>) -> bool
    where
        D: PartialEq<P>,
    {
        self.type_uri() == expected.type_uri()
            && self.title() == Some(expected.title())
            && self.status() == Some(expected.status())
            && self.detail() == expected.detail()
            && match (self.data(), expected.data()) {
                (None, None) => true,
                (Some(received), Some(expected)) => received == expected,
                _ => false,
            }
    }

    /// Problem identity, defaulting to `about:blank` when unavailable.
    pub fn type_uri(&self) -> &str {
        &self.type_uri
    }

    /// Borrow the received title, if present and correctly typed.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Received advisory status, if present.
    ///
    /// This does not replace the status of the actual HTTP response.
    pub const fn status(&self) -> Option<StatusCode> {
        self.status
    }

    /// Borrow the received public explanation, if available.
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// Borrow the received occurrence URI reference, if available.
    ///
    /// Relative references remain unresolved; the caller supplies the base URI.
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_deref()
    }

    /// Borrow explicitly selected public values.
    pub fn data(&self) -> Option<&D> {
        self.data.as_ref()
    }
}

impl<D> From<ProblemDetails<D>> for ProblemDocument<D> {
    /// Own the metadata and move the optional payload without serialization.
    fn from(details: ProblemDetails<D>) -> Self {
        Self {
            type_uri: details.type_uri.into(),
            title: Some(details.title.into()),
            status: Some(details.status),
            detail: details.detail,
            instance: details.instance,
            data: details.data,
        }
    }
}

/****************************************/
/* Report                               */
/****************************************/

/// Retains a typed error for public document and HTTP conversion.
/// Creating a report performs no serialization or diagnostic reporting. Borrow
/// or recover the original error to inspect its diagnostic message and sources.
/// With `actix-web` enabled, `Display` forwards to the original error to satisfy
/// Actix Web's response error contract. Public responses still use declared metadata.
#[must_use]
#[derive(Debug)]
pub struct Report<E: Problem> {
    problem: E,
    instance: Option<String>,
}

impl<E: Problem> Report<E> {
    /// Retain the original error and its declaration type.
    pub const fn new(problem: E) -> Self {
        Self {
            problem,
            instance: None,
        }
    }

    /// Attach a URI reference identifying this occurrence.
    /// Overrides the error's instance without changing the original error.
    /// The caller supplies a valid URI reference; syntax is not validated.
    /// Relative references require resolution against the document base URI.
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    /// Consume the report and construct an owned public document.
    /// Moves the attached instance rather than cloning it. The original error
    /// is dropped; inspect diagnostics before consuming the report.
    pub fn into_details(self) -> ProblemDetails<E::Data> {
        let definition = self.problem.definition();
        ProblemDetails {
            type_uri: definition.type_uri,
            title: definition.title,
            status: definition.status,
            detail: self.problem.detail(),
            instance: self.instance.or_else(|| self.problem.instance()),
            data: self.problem.into_data(),
        }
    }

    /// Borrow the original error, including its diagnostic source chain.
    pub const fn problem(&self) -> &E {
        &self.problem
    }

    /// Recover the original error without reporting it.
    /// Any instance attached to the report is discarded.
    pub fn into_problem(self) -> E {
        self.problem
    }

    /// Project public fields by reference. Serialize before dropping the report.
    /// Detail rendering and an attached instance still produce owned strings.
    ///
    /// ```
    /// # #[cfg(feature = "derive")]
    /// # {
    /// use problems::IntoReport;
    ///
    /// #[derive(Debug, thiserror::Error, problems::Problem)]
    /// #[error("private diagnostic")]
    /// #[problem(type_uri = "urn:example:retry", status = 429)]
    /// struct Retry {
    ///     #[problem(data)]
    ///     seconds: u32,
    /// }
    ///
    /// let report = Retry { seconds: 30 }.into_report();
    /// assert_eq!(report.as_details().data().unwrap().seconds, &30);
    ///
    /// let owned = report.into_details();
    /// assert_eq!(owned.data().unwrap().seconds, 30);
    /// # }
    /// ```
    pub fn as_details(&self) -> ProblemDetails<E::DataRef<'_>> {
        let definition = self.problem.definition();
        ProblemDetails {
            type_uri: definition.type_uri,
            title: definition.title,
            status: definition.status,
            detail: self.problem.detail(),
            instance: self.instance.clone().or_else(|| self.problem.instance()),
            data: self.problem.data(),
        }
    }
}

impl<E: Problem> From<E> for Report<E> {
    fn from(problem: E) -> Self {
        Self::new(problem)
    }
}

/****************************************/
/* Into Report                          */
/****************************************/

/// Converts a problem into a report while retaining its concrete error type.
/// Every type implementing `Problem` receives this convenience method.
/// This extension trait does not provide a framework response contract.
/// Use `Report<E>` as the handler error type to expose response and, with
/// `aide`, static output metadata implementations.
pub trait IntoReport: Problem + Sized {
    /// Wraps the original error without serialization or diagnostic reporting.
    fn into_report(self) -> Report<Self> {
        Report::from(self)
    }
}

impl<E: Problem> IntoReport for E {}

/****************************************/
/* Axum Response                        */
/****************************************/

#[cfg(feature = "axum")]
impl<E: Problem> axum::response::IntoResponse for Report<E> {
    /// Consumes the report and renders its declared public response.
    fn into_response(self) -> axum::response::Response {
        axum_response(self.problem().definition().status, self.into_details())
    }
}

#[cfg(feature = "axum")]
impl<E: Problem> axum::response::IntoResponse for &Report<E> {
    /// Renders an owned response while retaining the original error.
    fn into_response(self) -> axum::response::Response {
        axum_response(self.problem().definition().status, self.as_details())
    }
}

#[cfg(feature = "axum")]
fn axum_response<D: serde::Serialize>(
    status: StatusCode,
    details: ProblemDetails<D>,
) -> axum::response::Response {
    let headers = [(axum::http::header::CONTENT_TYPE, "application/problem+json")];
    axum::response::IntoResponse::into_response((status, headers, axum::Json(details)))
}

/****************************************/
/* Actix Web Response                   */
/****************************************/

#[cfg(feature = "actix-web")]
impl<E: Problem> std::fmt::Display for Report<E> {
    /// Forwards diagnostic formatting to the original error for Actix Web.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.problem(), formatter)
    }
}

#[cfg(feature = "actix-web")]
impl<E: Problem> actix_web::ResponseError for Report<E> {
    /// Converts the declared status to Actix Web's HTTP status type.
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::from_u16(self.problem().definition().status.as_u16())
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Returns the public JSON document without exposing diagnostic formatting.
    fn error_response(&self) -> actix_web::HttpResponse {
        actix_web::HttpResponse::build(self.status_code())
            .content_type("application/problem+json")
            .json(self.as_details())
    }
}

/****************************************/
/* Rocket Response                      */
/****************************************/

#[cfg(feature = "rocket")]
impl<'r, E: Problem> rocket::response::Responder<'r, 'static> for Report<E> {
    /// Consumes the report and builds an owned public response.
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        rocket_response(self.into_details(), request)
    }
}

#[cfg(feature = "rocket")]
impl<'r, E: Problem> rocket::response::Responder<'r, 'static> for &Report<E> {
    /// Builds a response whose body does not borrow the original report.
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        rocket_response(self.as_details(), request)
    }
}

#[cfg(feature = "rocket")]
fn rocket_response<D: serde::Serialize>(
    details: ProblemDetails<D>,
    request: &rocket::Request<'_>,
) -> rocket::response::Result<'static> {
    use rocket::response::Responder;

    // --- Let Rocket serialize the owned public document.
    let status = rocket::http::Status::new(details.status().as_u16());

    // --- Preserve its body while setting the declared status and problem media type.
    rocket::Response::build_from(rocket::serde::json::Json(details).respond_to(request)?)
        .status(status)
        .header(rocket::http::ContentType::new(
            "application",
            "problem+json",
        ))
        .ok()
}

/****************************************/
/* Poem Response                        */
/****************************************/

#[cfg(feature = "poem")]
impl<E: Problem + Send> poem::IntoResponse for Report<E>
where
    E::Data: Send,
{
    /// Consumes the report and renders its declared public response.
    fn into_response(self) -> poem::Response {
        poem_response(self.problem().definition().status, self.into_details())
    }
}

#[cfg(feature = "poem")]
impl<'a, E: Problem + Sync> poem::IntoResponse for &'a Report<E>
where
    E::DataRef<'a>: Send,
{
    /// Renders an owned response without consuming the diagnostic error.
    fn into_response(self) -> poem::Response {
        poem_response(self.problem().definition().status, self.as_details())
    }
}

#[cfg(feature = "poem")]
fn poem_response<D: serde::Serialize + Send>(
    status: StatusCode,
    details: ProblemDetails<D>,
) -> poem::Response {
    use poem::IntoResponse;
    poem::IntoResponse::into_response(
        poem::web::Json(details)
            .with_status(status)
            .with_content_type("application/problem+json"),
    )
}

#[cfg(feature = "poem")]
impl<E: Problem + Send> From<Report<E>> for poem::Error
where
    E::Data: Send,
{
    /// Consumes the diagnostic error and retains only its public response.
    /// Inspect or report diagnostics before converting.
    fn from(report: Report<E>) -> Self {
        Self::from_response(poem::IntoResponse::into_response(report))
    }
}

#[cfg(feature = "poem")]
impl<'a, E: Problem + Sync> From<&'a Report<E>> for poem::Error
where
    E::DataRef<'a>: Send,
{
    /// Retains only the public response in the Poem error; leaves the report available.
    fn from(report: &'a Report<E>) -> Self {
        Self::from_response(poem_response(
            report.problem().definition().status,
            report.as_details(),
        ))
    }
}

/****************************************/
/* Salvo Response                       */
/****************************************/

#[cfg(feature = "salvo")]
impl<E: Problem> salvo::Scribe for Report<E>
where
    E::Data: Send,
{
    /// Consumes the report and writes its declared public response.
    fn render(self, response: &mut salvo::Response) {
        salvo_response(
            self.problem().definition().status,
            self.into_details(),
            response,
        );
    }
}

#[cfg(feature = "salvo")]
impl<'a, E: Problem> salvo::Scribe for &'a Report<E>
where
    E::DataRef<'a>: Send,
{
    /// Writes public details while retaining the original report.
    fn render(self, response: &mut salvo::Response) {
        salvo_response(
            self.problem().definition().status,
            self.as_details(),
            response,
        );
    }
}

#[cfg(feature = "salvo")]
fn salvo_response<D: serde::Serialize + Send>(
    status: StatusCode,
    details: ProblemDetails<D>,
    response: &mut salvo::Response,
) {
    // --- Render the public document using Salvo's JSON writer.
    response.status_code(status);
    salvo::Scribe::render(salvo::writing::Json(details), response);
    // --- Replace the JSON writer's media type with the problem media type.
    response.headers_mut().insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("application/problem+json"),
    );
}

/****************************************/
/* Warp Response                        */
/****************************************/

#[cfg(feature = "warp")]
impl<E: Problem + Send> warp::Reply for Report<E> {
    /// Returns the public JSON document with its declared status and problem media type.
    fn into_response(self) -> warp::reply::Response {
        warp_response(self.problem().definition().status, self.into_details())
    }
}

#[cfg(feature = "warp")]
impl<E: Problem + Sync> warp::Reply for &Report<E> {
    /// Renders an owned response without cloning or consuming the original error.
    fn into_response(self) -> warp::reply::Response {
        warp_response(self.problem().definition().status, self.as_details())
    }
}

#[cfg(feature = "warp")]
fn warp_response<D: serde::Serialize>(
    status: StatusCode,
    details: ProblemDetails<D>,
) -> warp::reply::Response {
    warp::Reply::into_response(warp::reply::with_header(
        warp::reply::with_status(warp::reply::json(&details), status),
        "content-type",
        "application/problem+json",
    ))
}

/****************************************/
/* OpenAPI Documentation                */
/****************************************/

#[cfg(feature = "aide")]
impl<E: Problem> aide::OperationOutput for Report<E>
where
    E::Data: schemars::JsonSchema,
{
    type Inner = ProblemDetails<E::Data>;

    /// Describes the shared problem document schema and its JSON media type.
    fn operation_response(
        ctx: &mut aide::generate::GenContext,
        _: &mut aide::openapi::Operation,
    ) -> Option<aide::openapi::Response> {
        // --- Generate the shared document schema through Aide's schema context.
        let schema = ctx.schema.subschema_for::<ProblemDetails<E::Data>>();

        // --- Describe the JSON media type using that schema for every problem response.
        let mut response = aide::openapi::Response {
            description: "RFC 9457 Problem Details".into(),
            ..Default::default()
        };
        response.content.insert(
            "application/problem+json".into(),
            aide::openapi::MediaType {
                schema: Some(aide::openapi::SchemaObject {
                    json_schema: schema,
                    external_docs: None,
                    example: None,
                }),
                ..Default::default()
            },
        );
        Some(response)
    }

    /// Groups declared problem types by status without constructing errors.
    fn inferred_responses(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Vec<(Option<u16>, aide::openapi::Response)> {
        // --- Collect problem titles and identities by status, in ascending status order.
        let mut descriptions = std::collections::BTreeMap::<u16, Vec<String>>::new();
        for definition in E::definitions() {
            descriptions
                .entry(definition.status.as_u16())
                .or_default()
                .push(format!(
                    "{}\nProblem type: {}",
                    definition.title, definition.type_uri
                ));
        }

        // --- Build the common response schema once for all declared statuses.
        let Some(response) = Self::operation_response(ctx, operation) else {
            return Vec::new();
        };

        // --- Describe each status with its problem types while retaining the common schema.
        descriptions
            .into_iter()
            .map(|(status, descriptions)| {
                (
                    Some(status),
                    aide::openapi::Response {
                        description: descriptions.join("\n\n"),
                        ..response.clone()
                    },
                )
            })
            .collect()
    }
}
