//! Turn Rust errors into public HTTP error documents, following [RFC 9457].
//!
//! The library separates an error's diagnostic message and source chain from
//! the information sent to clients:
//!
//! 1. Implement [`Problem`] to declare the public type URI, title, status, and
//!    optional explanation (`detail`) or structured payload (`data`). The default
//!    `derive` feature provides `#[derive(Problem)]` for structs and enums.
//! 2. Wrap the error in a [`Report`] using [`IntoReport::into_report`]. The report
//!    keeps the original error available for logging or inspection.
//! 3. Use [`Report::as_details`] to borrow public data, or [`Report::into_details`]
//!    to move it into [`ProblemDetails`]. Serialize these details with Serde.
//!
//! To read a received document, deserialize a [`ProblemDocument<D>`], choosing
//! `D` to match its `data` payload.
//!
//! Optional `axum`, `actix-web`, `rocket`, `poem`, `salvo`, and `warp` features
//! let reports produce JSON responses with the declared HTTP status and
//! `application/problem+json` content type. `schemars` adds document schemas;
//! `aide` documents the declared response statuses and schema in OpenAPI.
//!
//! [RFC 9457]: https://www.rfc-editor.org/rfc/rfc9457.html

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

/// The public identity, summary, and HTTP status of a problem type.
///
/// A definition is shared by all occurrences of that problem. For example,
/// every name conflict has the same type URI and status, while its [`Problem::detail`]
/// can explain which name was taken. Reports and OpenAPI use the same definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProblemDefinition {
    /// Stable URI reference identifying the problem type (serialized as `type`).
    /// URI syntax is not validated.
    pub type_uri: &'static str,
    /// Short, human-readable summary shared by occurrences of this problem.
    pub title: &'static str,
    /// HTTP response status, also included in the public document.
    pub status: StatusCode,
}

/****************************************/
/* Problem                               */
/****************************************/

/// Declares which parts of a Rust error are public.
///
/// [`std::fmt::Display`] and [`std::error::Error::source`] provide diagnostics.
/// Reports build the public document from this trait's methods; they do not
/// copy the diagnostic message or source chain into it. Write `detail` for
/// clients and select only fields that are safe to expose as `data`.
///
/// # Deriving a problem
///
/// `#[derive(Problem)]` works on structs and enums with named, tuple, or unit
/// fields. It requires the `derive` feature, enabled by default. Implement
/// `Display` and `Error` separately, for example with `thiserror`.
///
/// ```rust
/// # #[cfg(feature = "derive")]
/// # {
/// use problems::{IntoReport, StatusCode};
///
/// // --- Declare the diagnostic error and its public explanation.
/// #[derive(Debug, thiserror::Error, problems::Problem)]
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
/// // --- Wrap an occurrence and build the public document.
/// let report = CreateProblem::NameConflict { name: "monthly".into() }.into_report();
/// let details = report.as_details();
/// assert_eq!(details.status(), StatusCode::CONFLICT);
/// assert_eq!(details.type_uri(), "urn:example:name-conflict");
/// assert_eq!(details.detail(), Some("The name 'monthly' is already in use."));
///
/// // --- Serialize the public fields; diagnostics remain on the original error.
/// let body = serde_json::to_value(&details)?;
/// assert_eq!(body["data"]["name"], "monthly");
/// assert!(body.get("instance").is_none());
/// assert_eq!(report.problem().to_string(), "duplicate name: monthly");
/// # }
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// # Public metadata
///
/// Put `#[problem(...)]` on a struct or on each enum variant:
///
/// - `type_uri = "..."` sets the stable identifier clients use to recognize the
///   problem. It is required unless an enum supplies a prefix.
/// - `status = 409` sets the HTTP status. Accepted values are 100–999; the default
///   is 500. Use `#[problem(409)]` as a separate attribute for the shorthand.
/// - `title = "..."` sets the summary. By default, the struct or variant name is
///   converted to Title Case: `NameConflict` becomes `Name Conflict`.
/// - `detail = "..."` formats an explanation using named fields or explicit
///   tuple indexes such as `{0}`. Diagnostic sources cannot be interpolated.
///   Without a format, there is no detail.
///
/// An enum's `#[problem(prefix = "urn:example")]` generates a type URI for each
/// local variant, such as `urn:example:name-conflict`. Prefixes ending in `:` or
/// `/` are used directly; other prefixes get a colon separator. A variant's
/// explicit `type_uri` overrides the generated URI. Use an explicit URI when
/// renaming a variant must preserve its public identity. Duplicate local URIs
/// are rejected.
///
/// The derive also generates definition constants: `DEFINITION` for a struct,
/// or the variant name in SHOUTY_SNAKE_CASE, such as `NAME_CONFLICT`, for an enum.
/// [`Problem::definitions`] lists them without constructing errors.
///
/// # Structured public data
///
/// `#[problem(data)]` includes a named field under `data`.
/// `#[problem(data = "public_name")]` changes its serialized name and is required
/// for tuple fields. Diagnostic source fields cannot be selected.
///
/// Selected fields produce a `<Type>Data` struct or enum, with one generic
/// parameter per selected field in declaration order. Borrowing uses references
/// as those parameters; consuming uses the original field types. Neither requires
/// cloning the error. A problem with no public data uses `()` for its payload
/// types and omits `data`. Generated data enums are Serde-untagged: their serialized
/// payloads contain no variant tag.
///
/// # Combining problems
///
/// A single-field tuple variant marked `#[problem(transparent)]` forwards its
/// definition, detail, instance, and data to the wrapped `Problem`. Its definitions
/// join the enclosing enum's list in variant order. It has no local definition
/// constant or separate public metadata.
///
/// # Manual implementation
///
/// Without the derive, implement the required metadata methods and payload types.
/// Use `()` for both payload types when there is no public data. The optional
/// `detail`, `instance`, `data`, and `into_data` methods default to `None`.
///
/// ```rust
/// use problems::{IntoReport, Problem, ProblemDefinition, StatusCode};
///
/// // --- Define an error and the metadata shared by all its occurrences.
/// #[derive(Debug, thiserror::Error)]
/// #[error("record not found")]
/// struct MissingRecord;
///
/// const MISSING: ProblemDefinition = ProblemDefinition {
///     type_uri: "urn:example:missing-record",
///     title: "Record not found",
///     status: StatusCode::NOT_FOUND,
/// };
///
/// // --- Implement the public contract without a payload.
/// impl Problem for MissingRecord {
///     type Data = ();
///     type DataRef<'a> = ();
///
///     fn definition(&self) -> &'static ProblemDefinition {
///         &MISSING
///     }
///
///     fn definitions() -> impl Iterator<Item = &'static ProblemDefinition> {
///         std::iter::once(&MISSING)
///     }
/// }
///
/// // --- Use the same report API as a derived problem.
/// let details = MissingRecord.into_report().into_details();
/// assert_eq!(details.status(), StatusCode::NOT_FOUND);
/// assert!(details.data().is_none());
/// ```
pub trait Problem: std::error::Error {
    /// Payload returned by [`Self::into_data`]. Use `()` when there is no data.
    type Data: serde::Serialize
    where
        Self: Sized;

    /// Payload returned by [`Self::data`], usually containing references to fields.
    /// Use `()` when there is no data.
    type DataRef<'a>: serde::Serialize
    where
        Self: Sized + 'a;

    /// Borrow the structured public payload. The default is no payload.
    fn data(&self) -> Option<Self::DataRef<'_>>
    where
        Self: Sized,
    {
        None
    }

    /// Consume the error to obtain its public payload. The default is no payload.
    fn into_data(self) -> Option<Self::Data>
    where
        Self: Sized,
    {
        None
    }

    /// Definition of the problem represented by this error occurrence.
    fn definition(&self) -> &'static ProblemDefinition;

    /// List every definition this error type can return, without constructing errors.
    ///
    /// Include every possible result of [`Self::definition`]. OpenAPI uses this
    /// list to document response statuses and problem types.
    fn definitions() -> impl Iterator<Item = &'static ProblemDefinition>
    where
        Self: Sized;

    /// Client-facing explanation of this occurrence. The default is no detail.
    fn detail(&self) -> Option<String> {
        None
    }

    /// URI reference identifying this occurrence. The default is no instance.
    fn instance(&self) -> Option<String> {
        None
    }
}

/****************************************/
/* Problem Details                      */
/****************************************/

/// A public error document ready to serialize with Serde.
///
/// Obtain it from [`Report::as_details`] or [`Report::into_details`]. `D` is the
/// type of the structured `data` payload; it may contain borrowed fields.
/// The document always includes `type`, `title`, and a numeric `status`.
/// Optional `detail`, `instance`, and `data` members are omitted when absent.
///
/// Equality compares every member, including `instance`. To deserialize a
/// received body, use [`ProblemDocument`] instead.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ProblemDetails<D = ()> {
    /// URI reference identifying the problem type, serialized as `type`.
    #[serde(rename = "type")]
    type_uri: &'static str,

    /// Human-readable summary shared by occurrences of this problem type.
    title: &'static str,

    /// Declared HTTP status, serialized as an integer.
    #[serde(serialize_with = "serialize_status")]
    #[cfg_attr(
        feature = "schemars",
        schemars(with = "u16", range(min = 100, max = 999))
    )]
    status: StatusCode,

    /// Explanation for this occurrence. Omitted when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,

    /// URI reference identifying this occurrence. Omitted when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    instance: Option<String>,

    /// Structured payload selected for clients. Omitted when absent.
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

    /// URI reference identifying the problem type (the document's `type` member).
    pub const fn type_uri(&self) -> &'static str {
        self.type_uri
    }

    /// Human-readable summary of the problem type.
    pub const fn title(&self) -> &'static str {
        self.title
    }

    /// Borrow the public explanation for this occurrence, if present.
    /// Clients should use the type URI and data for programmatic handling.
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// Borrow the URI reference identifying this occurrence, if present.
    /// It need not point to a retrievable resource.
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_deref()
    }

    /// Borrow the structured public payload, if present.
    pub fn data(&self) -> Option<&D> {
        self.data.as_ref()
    }
}

/****************************************/
/* Problem Document                     */
/****************************************/

/// A received error document, deserialized with Serde.
///
/// Choose `D` to match the structured `data` payload. Use `()` when no payload
/// is expected, a known struct for typed fields, or `serde_json::Value` for
/// arbitrary JSON data. The library does not choose a wire format: the Serde
/// decoder and payload type must both support the incoming representation.
///
/// A missing `type` becomes `about:blank`. Other missing members are `None`;
/// JSON `null` also means no optional value. Unknown members are discarded.
/// Incorrect member types fail deserialization, rather than being ignored as
/// RFC 9457 recommends. A provided status must be an integer in 100–999.
///
/// Use [`Self::is_type`] to recognize a problem by its URI, or [`Self::matches`]
/// to compare its public fields with expected producer details. `matches` ignores
/// `instance`; equality includes it. This type contains public values only,
/// with no original Rust error or source chain.
///
/// # Reading typed data from JSON or XML
///
/// This example uses the same payload struct for both formats. XML support
/// depends on the decoder's mapping; not every JSON payload has an equivalent
/// XML representation.
///
/// ```rust
/// use problems::{ProblemDocument, StatusCode};
///
/// // --- Describe the public payload expected from the server.
/// #[derive(Debug, PartialEq, serde::Deserialize)]
/// struct RetryData {
///     seconds: u32,
/// }
/// type RetryDocument = ProblemDocument<RetryData>;
///
/// // --- Decode each format using its Serde decoder.
/// let json: RetryDocument = serde_json::from_str(r#"{
///     "type": "urn:example:retry",
///     "title": "Retry later",
///     "status": 429,
///     "data": {"seconds": 30}
/// }"#)?;
/// let xml: RetryDocument = serde_xml_rs::from_str(r#"
///     <problem xmlns="urn:ietf:rfc:7807">
///         <type>urn:example:retry</type>
///         <title>Retry later</title>
///         <status>429</status>
///         <data><seconds>30</seconds></data>
///     </problem>
/// "#)?;
///
/// // --- Work with the same typed values, regardless of the format.
/// assert_eq!(json, xml);
/// assert_eq!(json.status(), Some(StatusCode::TOO_MANY_REQUESTS));
/// assert_eq!(json.data(), Some(&RetryData { seconds: 30 }));
/// assert_eq!(json.detail(), None);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ProblemDocument<D = ()> {
    /// Received `type` URI, defaulting to `about:blank` when absent.
    #[serde(rename = "type", default = "default_type")]
    type_uri: String,

    /// Received summary, if present.
    title: Option<String>,

    /// Received status, if present, decoded from an integer.
    #[serde(default, deserialize_with = "deserialize_status")]
    #[cfg_attr(
        feature = "schemars",
        schemars(with = "Option<u16>", range(min = 100, max = 999))
    )]
    status: Option<StatusCode>,

    /// Received explanation for this occurrence, if present.
    detail: Option<String>,

    /// Received occurrence URI reference, if present. Stored without resolution.
    instance: Option<String>,

    /// Received structured payload decoded as `D`, if present.
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
    /// Check whether the received `type` URI equals a declared problem's URI.
    ///
    /// Other members do not affect this check. URI references are compared as
    /// strings; resolve relative references yourself when necessary.
    ///
    /// ```rust
    /// # #[cfg(feature = "derive")]
    /// # {
    /// use problems::ProblemDocument;
    ///
    /// // --- Declare the problem types the client recognizes.
    /// #[derive(Debug, thiserror::Error, problems::Problem)]
    /// #[problem(prefix = "urn:example")]
    /// enum CreateProblem {
    ///     #[error("duplicate name")]
    ///     #[problem(409)]
    ///     NameConflict,
    ///     #[error("resource missing")]
    ///     #[problem(404)]
    ///     Missing,
    /// }
    ///
    /// // --- Read a response containing only the problem identifier.
    /// let received: ProblemDocument = serde_json::from_str(
    ///     r#"{"type": "urn:example:name-conflict"}"#,
    /// )?;
    ///
    /// // --- Recognize it without constructing an error occurrence.
    /// assert!(received.is_type(&CreateProblem::NAME_CONFLICT));
    /// assert!(!received.is_type(&CreateProblem::MISSING));
    /// # }
    /// # Ok::<(), serde_json::Error>(())
    /// ```
    pub fn is_type(&self, definition: &ProblemDefinition) -> bool {
        self.type_uri == definition.type_uri
    }

    /// Check whether public fields match expected details, ignoring `instance`.
    ///
    /// Type URI, title, status, detail, and data must all match. A missing member
    /// is not a wildcard. Payload types may differ if `D` implements `PartialEq<P>`.
    /// To include the occurrence URI, convert the details with
    /// [`ProblemDocument::from`] and compare using `==`.
    ///
    /// ```rust
    /// # #[cfg(feature = "derive")]
    /// # {
    /// use problems::{IntoReport, ProblemDocument};
    ///
    /// // --- Declare a conflict with a public explanation.
    /// #[derive(Debug, thiserror::Error, problems::Problem)]
    /// #[problem(prefix = "urn:example")]
    /// enum CreateProblem {
    ///     #[error("duplicate name: {name}")]
    ///     #[problem(status = 409, detail = "The name '{name}' is already in use.")]
    ///     NameConflict { name: String },
    /// }
    ///
    /// // --- Build the failure expected by a client or an API test.
    /// let expected = CreateProblem::NameConflict { name: "monthly".into() }
    ///     .into_report()
    ///     .with_instance("/occurrences/1")
    ///     .into_details();
    ///
    /// // --- Decode the same failure from a different occurrence.
    /// let received: ProblemDocument = serde_json::from_str(r#"{
    ///     "type": "urn:example:name-conflict",
    ///     "title": "Name Conflict",
    ///     "status": 409,
    ///     "detail": "The name 'monthly' is already in use.",
    ///     "instance": "/occurrences/2"
    /// }"#)?;
    /// assert!(received.matches(&expected));
    /// assert_ne!(received, ProblemDocument::from(expected));
    ///
    /// // --- A different explanation makes it a different failure.
    /// let different_name = CreateProblem::NameConflict { name: "weekly".into() }
    ///     .into_report()
    ///     .into_details();
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

    /// Received `type` URI, or `about:blank` if it was absent.
    /// `about:blank` means the HTTP status alone describes the problem.
    pub fn type_uri(&self) -> &str {
        &self.type_uri
    }

    /// Borrow the received title, if present.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Received advisory status, if present.
    ///
    /// This does not replace the status of the actual HTTP response.
    pub const fn status(&self) -> Option<StatusCode> {
        self.status
    }

    /// Borrow the received explanation, if present.
    /// Clients should use the type URI and data for programmatic handling.
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// Borrow the received occurrence URI reference, if available.
    ///
    /// Relative references remain unresolved; the caller supplies the base URI.
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_deref()
    }

    /// Borrow the structured public payload, if present.
    pub fn data(&self) -> Option<&D> {
        self.data.as_ref()
    }
}

impl<D> From<ProblemDetails<D>> for ProblemDocument<D> {
    /// Convert producer details into a received-document value without serialization.
    ///
    /// Metadata strings become owned; the payload is moved as-is, so any references
    /// inside it still borrow their original values.
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

/// Keeps the original error while preparing its public document or HTTP response.
///
/// Create a report with [`Self::new`] or [`IntoReport::into_report`]. Creation
/// does not serialize, log, or otherwise report the error. Inspect diagnostics
/// through [`Self::problem`], or recover the error with [`Self::into_problem`].
///
/// [`Self::as_details`] borrows selected public data; [`Self::into_details`]
/// moves it out. With a framework feature enabled, return `Report<E>` as a
/// handler's error type to produce a public JSON response. Actix Web also
/// requires `Display`, which forwards to the original error's diagnostic message.
#[must_use]
#[derive(Debug)]
pub struct Report<E: Problem> {
    problem: E,
    instance: Option<String>,
}

impl<E: Problem> Report<E> {
    /// Wrap an error without serializing or logging it.
    pub const fn new(problem: E) -> Self {
        Self {
            problem,
            instance: None,
        }
    }

    /// Set the public `instance` URI reference, such as `/occurrences/123`.
    ///
    /// This overrides [`Problem::instance`] without changing the error. URI
    /// syntax is not validated; relative references require a base URI to resolve.
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    /// Consume the report and move its selected public data into a document.
    ///
    /// The attached instance is also moved. The original error is consumed, so
    /// inspect or log diagnostics first. See [`Self::as_details`] for a comparison
    /// of borrowed and owned data.
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

    /// Consume the report and recover the original error.
    /// Any instance attached to the report is discarded.
    pub fn into_problem(self) -> E {
        self.problem
    }

    /// Build public details that borrow the error's selected data fields.
    ///
    /// The report remains available for diagnostics and must outlive the borrowed
    /// data. No error fields are cloned. Rendering `detail` and obtaining
    /// `instance` may still allocate strings; an attached instance is cloned.
    /// Use [`Self::into_details`] when the document needs to own its data.
    ///
    /// ```rust
    /// # #[cfg(feature = "derive")]
    /// # {
    /// use problems::IntoReport;
    ///
    /// // --- Select a value that clients can use to schedule a retry.
    /// #[derive(Debug, thiserror::Error, problems::Problem)]
    /// #[error("rate limit reached")]
    /// #[problem(type_uri = "urn:example:retry", status = 429)]
    /// struct Retry {
    ///     #[problem(data)]
    ///     seconds: u32,
    /// }
    ///
    /// // --- Borrow public data while keeping the original error available.
    /// let report = Retry { seconds: 30 }.into_report();
    /// let borrowed = report.as_details();
    /// assert_eq!(borrowed.data().unwrap().seconds, &30);
    /// let body = serde_json::to_value(&borrowed)?;
    /// assert_eq!(body["data"]["seconds"], 30);
    /// assert_eq!(report.problem().seconds, 30);
    ///
    /// // --- Move the data into a document when the report is no longer needed.
    /// let owned = report.into_details();
    /// assert_eq!(owned.data().unwrap().seconds, 30);
    /// # }
    /// # Ok::<(), serde_json::Error>(())
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

/// Adds `.into_report()` to every type implementing [`Problem`].
///
/// Import this trait to wrap an error in a [`Report`] without losing its concrete
/// type. Use `Report<E>` as a handler's error type when returning framework
/// responses; this convenience trait itself only wraps the error.
pub trait IntoReport: Problem + Sized {
    /// Wrap the error without serializing or logging it.
    fn into_report(self) -> Report<Self> {
        Report::from(self)
    }
}

impl<E: Problem> IntoReport for E {}

/****************************************/
/* Axum Response                        */
/****************************************/

/// Convert a report into an Axum response with its public JSON body.
///
/// ```rust
/// # #[cfg(all(feature = "derive", feature = "axum"))]
/// # {
/// use axum::response::IntoResponse;
/// use problems::{IntoReport, Report, StatusCode};
///
/// // --- Declare the error returned by the handler.
/// #[derive(Debug, thiserror::Error, problems::Problem)]
/// #[error("record missing")]
/// #[problem(type_uri = "urn:example:missing", status = 404)]
/// struct MissingRecord;
///
/// async fn get_record() -> Result<String, Report<MissingRecord>> {
///     Err(MissingRecord.into_report())
/// }
///
/// // --- Axum converts the handler's report into a problem response.
/// let response = MissingRecord.into_report().into_response();
/// assert_eq!(response.status(), StatusCode::NOT_FOUND);
/// assert_eq!(response.headers()["content-type"], "application/problem+json");
/// # }
/// ```
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
