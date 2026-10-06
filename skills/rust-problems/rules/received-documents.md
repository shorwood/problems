# Received documents

## Prefer

- Shared producer/client contract → reuse generated `<Type>Data<owned field types>` when available; avoids duplicate DTO. Independent/external client contract → small typed `Deserialize` payload. Unknown/exploratory JSON → `serde_json::Value`. No expected data → default `()`.
- Recognize via `is_type(&Type::DEFINITION)` or variant constant; no constructed error just for URI comparison. Inspect typed data for action, prose for display.
- Test public fields while ignoring occurrence URI → `matches`. Shared generated payload → owned expected details with same payload parameters; derived `PartialEq` supports matching. Borrowed/owned parameters or separate client DTO may lack cross-type `PartialEq`; assert getters then. Full document including instance → equality.
- Preserve actual HTTP status alongside received body. Handle decode errors explicitly; malformed member types are not silently ignored by this library.

Deserialize `ProblemDocument<D>` via app Serde decoder. Default `D = ()`; expected data needs typed `Deserialize` or `serde_json::Value`. Public values only; no original error/source chain. XML/other formats depend on decoder mapping. No built-in HTTP client/format negotiation.

```rust
use problems::ProblemDocument;

#[derive(Debug, serde::Deserialize)]
struct RetryData { seconds: u32 }

let received: ProblemDocument<RetryData> = serde_json::from_str(r#"{
    "type": "urn:example:retry", "status": 429, "data": {"seconds": 30}
}"#)?;
assert_eq!(received.data().unwrap().seconds, 30);
assert_eq!(received.status().unwrap().as_u16(), 429);
assert_eq!(received.detail(), None);
# Ok::<(), serde_json::Error>(())
```

- Missing type → `about:blank`; missing optional members/optional JSON null → `None`. Unknown members discarded. Wrong member type or supplied status outside 100–999 → decode error.
- `is_type(&definition)`: URI string equality only. Relative URI remains unresolved.
- `matches(&details)`: compare type/title/status/detail/data; ignore instance. Missing member not wildcard. Payload comparison needs `D: PartialEq<P>`.
- `ProblemDocument::from(details)`: no serialization; strings become owned, payload moves as-is. `==` includes instance.
- Received status advisory; inspect actual HTTP response status too. Programmatic handling: type/data, not parsed title/detail prose.

```rust
use problems::{IntoReport, Problem, ProblemDocument};

#[derive(Debug, thiserror::Error, Problem)]
#[error("record missing")]
#[problem(type_uri = "urn:example:missing", title = "Missing", status = 404)]
struct Missing;

let expected = Missing.into_report().with_instance("/occurrences/1").into_details();
let received: ProblemDocument = serde_json::from_str(r#"{
    "type": "urn:example:missing", "title": "Missing", "status": 404,
    "instance": "/occurrences/2"
}"#)?;
assert!(received.is_type(&Missing::DEFINITION));
assert!(received.matches(&expected));
assert_ne!(received, ProblemDocument::from(expected));
# Ok::<(), serde_json::Error>(())
```

Shared contract; no duplicate client payload:

```rust
use problems::{IntoReport, Problem, ProblemDocument};

#[derive(Debug, thiserror::Error, Problem)]
#[error("rate limit")]
#[problem(type_uri = "urn:example:retry", status = 429)]
struct Retry {
    #[problem(data)]
    seconds: u32,
}

let expected = Retry { seconds: 30 }.into_report().into_details();
let received: ProblemDocument<RetryData<u32>> =
    serde_json::from_value(serde_json::to_value(&expected)?)?;
assert!(received.matches(&expected));
assert_eq!(received.data().unwrap().seconds, 30);
# Ok::<(), serde_json::Error>(())
```
