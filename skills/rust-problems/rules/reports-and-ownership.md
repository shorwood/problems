# Reports + ownership

## Prefer

- Helpers return concrete problem `E`; HTTP boundary returns `Report<E>`. Wrap once. Same error family → `?`; explicit constructed error → `.into_report()`; final existing `Result<T, E>` → `.map_err(IntoReport::into_report)`.
- Retain diagnostics or serialize while report lives → `as_details()`. Hand off standalone document → `into_details()`. Returning report directly to adapter needs neither projection nor cloning.
- Several problem families → existing aggregate `E` when available. Add transparent boundary enum only when typed aggregation/inference helps. `From` conversions do not chain automatically: inner error → aggregate → report needs explicit intermediate step.
- Attach occurrence URI at boundary where request context exists. Inspect/log original error before consuming; preserve app logging policy, no extra logging layer just for reports.

Wrap: `.into_report()` via `IntoReport`, `Report::new(error)`, or `From<E>`. Last path enables `?` into `Result<T, Report<E>>`. Unrelated errors still need app classification/wrapper.

- `problem()` borrows diagnostics; `into_problem()` recovers error, drops report instance context.
- `as_details()` borrows selected data; `into_details()` moves data. Neither needs error/data clone. Detail allocates string; borrowed projection may clone instance text. Borrowed details cannot outlive report.
- `with_instance(uri)` overrides problem instance. Local derive defaults absent; manual/transparent problem can supply instance.
- Serialize `ProblemDetails<D>`, not diagnostic error. Getters inspect document. Required: type/title/numeric status. Absent detail/instance/data omitted.

```rust
use problems::{IntoReport, Problem};

#[derive(Debug, serde::Serialize)]
struct PublicValue(u32); // No Clone.

#[derive(Debug, thiserror::Error, Problem)]
#[error("private failure")]
#[problem(type_uri = "urn:example:failure")]
struct Failure {
    #[problem(data)]
    value: PublicValue,
}

let report = Failure { value: PublicValue(7) }
    .into_report().with_instance("/occurrences/123");
{
    let borrowed = report.as_details();
    assert_eq!(borrowed.data().unwrap().value.0, 7);
    assert_eq!(borrowed.instance(), Some("/occurrences/123"));
}
assert_eq!(report.problem().value.0, 7);
let owned = report.into_details();
assert_eq!(owned.data().unwrap().value.0, 7);
```
