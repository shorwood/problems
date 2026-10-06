# Runtime status

## Prefer

- Fixed variant status for app-classified failures: fewer runtime states, precise OpenAPI inference. Field-derived status when variable code belongs to intended public contract, not as replacement for classification.
- Already validated code → `problems::StatusCode`. Existing integer code → keep original type; marker performs checked conversion. No preemptive `as u16`, manual range check, or duplicated fallback.
- Read `problem.status()` for occurrence decisions; definition constants for static identity/docs. Let report/adapter carry status; avoid independent outer status override.

One `#[problem(status)]` field per struct/nontransparent enum variant. Bound: `Copy + TryInto<u16>`. Signed/unsigned 8–64-bit integers, `problems::StatusCode`, aliases, compatible custom types work.

Checked conversion → validate 100–999 → fallback **500** on either failure. No narrowing cast. Marker does not select data. Reject diagnostic source, second status field, explicit declaration status, or transparent field marker.

```rust
use problems::{IntoReport, Problem};

#[derive(Debug, thiserror::Error, Problem)]
#[error("upstream failure")]
#[problem(type_uri = "urn:example:upstream")]
struct Upstream {
    #[problem(status)]
    code: i64,
}

for (code, expected) in [(100, 100), (503, 503), (999, 999),
                         (-1, 500), (1000, 500), (65536 + 418, 500)] {
    let report = Upstream { code }.into_report();
    assert_eq!(report.problem().status().as_u16(), expected);
    assert_eq!(report.as_details().status().as_u16(), expected);
    assert_eq!(report.into_details().status().as_u16(), expected);
}
assert_eq!(Upstream::DEFINITION.status.as_u16(), 500);
```

`Problem::status()` = occurrence status, used by reports + HTTP adapters. Definition constants + `definition().status` + `definitions()` stay static **500** for field-derived declarations. Aide infers 500, not all runtime codes. Need precise inferred statuses? Choose fixed declaration statuses.
