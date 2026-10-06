# Detail + data

## Prefer

- Detail explains client recovery; data carries machine-actionable values. Same field may serve both. Omit detail/data when they add no information; no duplicated status/title inside data.
- Select existing safe fields directly; avoid parallel response DTO + hand-built JSON for same contract. Named fields + default data names first; rename only for intended wire name.
- Keep diagnostic context on original error. Sanitized public values separate from source strings. JSON consumers use type/data, never parse detail. [RFC 9457, detail guidance](https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.4).

- `detail = "Name: {name}"` = standalone `#[problem("Name: {name}")]`. Shorthand metadata needs separate attribute. Detail reference does not select `data`.
- Named placeholders or explicit tuple indexes `{0}`. Formatting traits, literal width/precision, `{{`/`}}` supported. No implicit tuple `{}` or dynamic width/precision.
- Named field `#[problem(data)]`; rename with `data = "public_name"`. Tuple data requires explicit name. Selected fields require `Serialize`; generated payload enums untagged, no variant discriminator. Containers derive Debug/Clone/PartialEq/Eq/Serialize/Deserialize with applicable generic bounds; non-Clone field still usable when no cloning requested.
- Diagnostic sources: field named `source`, `#[source]`, `#[from]`, or `source` token inside field's `#[error(...)]`. Never select as detail/data/status. Expose separate public value.

```rust
use problems::{IntoReport, Problem};

#[derive(Debug, thiserror::Error, Problem)]
#[error("private failure for {name}: {source}")]
#[problem(type_uri = "urn:example:name-conflict", status = 409)]
#[problem("Name '{name}' conflicts; request {request_id:04x}.")]
struct Conflict {
    #[problem(data = "requested_name")]
    name: String,
    request_id: u16,
    source: std::io::Error,
}

let report = Conflict {
    name: "monthly".into(),
    request_id: 42,
    source: std::io::Error::other("SECRET"),
}.into_report();
let body = serde_json::to_value(report.as_details())?;
assert_eq!(body["detail"], "Name 'monthly' conflicts; request 002a.");
assert_eq!(body["data"], serde_json::json!({"requested_name": "monthly"}));
assert!(!body.to_string().contains("SECRET"));
# Ok::<(), serde_json::Error>(())
```

Tuple syntax:

```rust
use problems::Problem;

#[derive(Debug, thiserror::Error, Problem)]
#[error("failure")]
#[problem(type_uri = "urn:example:retry", detail = "Retry in {0} seconds.")]
struct Retry(#[problem(data = "seconds")] u32);

assert_eq!(Retry(30).detail().as_deref(), Some("Retry in 30 seconds."));
```
