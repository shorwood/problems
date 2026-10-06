#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    #[problem(status = 418)]
    Conflict { #[problem(status)] code: u32 },
    #[error("failure")]
    Duplicate { #[problem(status)] a: u32, #[problem(status)] b: u32 },
    #[error("failure")]
    Repeated { #[problem(status)] #[problem(status)] code: u32 },
    #[error("failure")]
    Source { #[problem(status)] source: std::io::Error },
    #[error(transparent)]
    #[problem(transparent)]
    Transparent(#[problem(status)] std::io::Error),
    #[error("failure")]
    #[problem("first")]
    #[problem(detail = "second")]
    Detail,
    #[error("failure")]
    #[problem(detail = "first")]
    #[problem("second")]
    ReverseDetail,
    #[error("failure")]
    #[problem("first")]
    #[problem("second")]
    RepeatedDetail,
    #[error("failure")]
    #[problem("{missing}")]
    UnknownDetail { value: String },
    #[error("failure")]
    #[problem("{source}")]
    SourceDetail { source: std::io::Error },
}

#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("failure")]
#[problem(type_uri = "urn:test:unsupported")]
struct Unsupported(#[problem(status)] std::time::Duration);

#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("failure")]
#[problem(type_uri = "urn:test:non-copy")]
struct NonCopy(#[problem(status)] String);

fn main() {}
