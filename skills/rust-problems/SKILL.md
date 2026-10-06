---
name: rust-problems
description: Use the Rust problems crate to declare RFC 9457 errors, render framework responses, generate Aide OpenAPI metadata, or decode typed problem documents. Apply when integrating or troubleshooting this crate; not for general Rust error handling.
metadata:
  library-version: "0.1.1"
---

# Rust Problems 0.1.1

Check resolved version + enabled features first. Target: current repository capabilities, treated as 0.1.1. Preserve app error classification + public contract.

`Problem` = public contract. `Report<E>` = retained diagnostic error. `ProblemDetails<D>` = outgoing document. `ProblemDocument<D>` = received document. Public exposure explicit; framework rejection handling app-owned.

Rules distinguish API constraints from preferred defaults. Defaults yield to user intent, established project style, and existing wire contracts.

## Read matching rules only

| Task | Read |
| --- | --- |
| Struct/enum metadata, stable URI, transparent wrapper, manual impl | [Declarations](rules/declarations.md) |
| Detail formatting, public data, private sources | [Detail + data](rules/detail-and-data.md) |
| Field-derived status, fallback, static definitions | [Runtime status](rules/runtime-status.md) |
| Wrap error, borrow/move payload, keep diagnostics, set instance | [Reports + ownership](rules/reports-and-ownership.md) |
| Framework response, rejection adapter, features, Aide schema | [Frameworks + OpenAPI](rules/frameworks-and-openapi.md) |
| Decode payload, recognize type, compare documents | [Received documents](rules/received-documents.md) |

## Ground + verify

Source checkout: `crates/problems/src/lib.rs` = runtime/adapters; `crates/problems-derive/src/lib.rs` = grammar; `examples/<framework>/src/lib.rs` = integration patterns. Elsewhere: resolved version's [runtime docs](https://docs.rs/problems) + [derive docs](https://docs.rs/problems-derive). Avoid assuming every 0.1 release supports these APIs.

App changes: enable chosen framework; check HTTP status + media type + JSON + private-field absence. Library changes: existing integration/UI tests. Consumer task: stay within existing public API.
