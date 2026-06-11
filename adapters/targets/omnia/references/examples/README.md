# Omnia Worked Examples

Index of the worked-example corpus for Omnia crate generation. Pick the one file that matches the task at hand; do not read the tree wholesale.

## crates/ — crate-shape examples

| File | Read when |
| --- | --- |
| [crates/single-handler.md](crates/single-handler.md) | Generating a crate with one handler (the `r9k-adapter` walkthrough). |
| [crates/multi-handler.md](crates/multi-handler.md) | Generating a crate with several handlers sharing domain logic (the `cars` walkthrough). |
| [crates/anti-patterns.md](crates/anti-patterns.md) | Reviewing generated code for known bad shapes before they ship. |

### crates/capabilities/ — one example per capability trait

| File | Read when the spec requires |
| --- | --- |
| [crates/capabilities/http-request.md](crates/capabilities/http-request.md) | Outbound HTTP calls (`HttpRequest`). |
| [crates/capabilities/config.md](crates/capabilities/config.md) | Environment configuration (`Config`). |
| [crates/capabilities/statestore.md](crates/capabilities/statestore.md) | Caching / key-value state (`StateStore`). |
| [crates/capabilities/publisher.md](crates/capabilities/publisher.md) | Publishing messages or events (`Publish`). |
| [crates/capabilities/broadcast.md](crates/capabilities/broadcast.md) | Pushing data to WebSocket clients (`Broadcast`). |
| [crates/capabilities/identity.md](crates/capabilities/identity.md) | Bearer-token authentication (`Identity`). |
| [crates/capabilities/blobstore.md](crates/capabilities/blobstore.md) | Binary blob storage (`Blobstore`). |
| [crates/capabilities/documentstore.md](crates/capabilities/documentstore.md) | JSON document storage / queries (`DocumentStore`). |
| [crates/capabilities/tablestore.md](crates/capabilities/tablestore.md) | Tabular storage (`TableStore`). |

### crates/updates/ — change-classification examples

| File | Read when the slice is classified |
| --- | --- |
| [crates/updates/additive.md](crates/updates/additive.md) | Additive — a new handler joins an existing crate. |
| [crates/updates/modifying.md](crates/updates/modifying.md) | Modifying — business logic changes inside existing handlers. |
| [crates/updates/structural.md](crates/updates/structural.md) | Structural — the domain model is refactored. |
| [crates/updates/subtractive.md](crates/updates/subtractive.md) | Subtractive — an endpoint or handler is removed. |

## replay/ — replay-harness examples

| File | Read when |
| --- | --- |
| [replay/handler.md](replay/handler.md) | Writing the replay handler entry point for a crate. |
| [replay/fixtures.md](replay/fixtures.md) | Authoring replay text fixtures from captured traffic. |
| [replay/tests.md](replay/tests.md) | Wiring replay fixtures into the crate's test suite. |

## tests/ — capability test walkthroughs

| File | Read when testing |
| --- | --- |
| [tests/testing.md](tests/testing.md) | Any crate — the general testing-pattern overview; start here. |
| [tests/testing-http.md](tests/testing-http.md) | A simple HTTP handler (Example 01). |
| [tests/testing-statestore.md](tests/testing-statestore.md) | Cache behavior over `StateStore` (Example 02). |
| [tests/testing-publisher.md](tests/testing-publisher.md) | Messaging over `Publish` (Example 03). |
| [tests/testing-blobstore.md](tests/testing-blobstore.md) | Blob storage over `Blobstore` (Example 04). |
| [tests/testing-documentstore.md](tests/testing-documentstore.md) | Document storage over `DocumentStore` (Example 05). |
