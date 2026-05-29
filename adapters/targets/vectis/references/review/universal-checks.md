# Universal Codex Checks — Rust/Crux Heuristics

Read this at step 2c of the review-fix cycle, after the specialists complete and before the antagonist runs.

The lead applies every `UNI-*` rule from [`adapters/shared/rules/universal/`](../../../../shared/rules/universal/). Several universal checks overlap with categories already covered by the specialists; skip those:

| Universal check | Already covered by | Action |
|---|---|---|
| UNI-002 Unvalidated input | CRX-002, LOG-007 | Skip |
| UNI-003 Serialization failures | CRX-005, GEN-009 | Skip |
| UNI-004 Logic bugs | LOG-001..008 | Skip |
| UNI-006 Race conditions | LOG-003, LOG-006 | Skip |
| UNI-010 Panics/crashes | GEN-001, CRX-011 | Skip |
| UNI-017 Type safety (partial) | CRX-008 | Apply beyond ViewModel |
| UNI-018 Hardcoded secrets | GEN-003 | Skip |

Apply the remaining checks with these Rust-specific heuristics:

- **UNI-001** (uninitialised values): Look for `#[derive(Default)]` on structs where the default value has no valid domain meaning. Check `Option::None` fields accessed without distinguishing "not loaded" from "intentionally empty".
- **UNI-005** (unbounded growth): Look for `Vec` or `VecDeque` fields that receive `.push()` without corresponding `.remove()`, `.retain()` bounds, or capacity limits. Check for `Command` futures that are never cancelled.
- **UNI-007** (chatty calls): Look for duplicate `HttpRequest` calls fetching the same data, SSE reconnect handlers that re-fetch data already delivered by the SSE event, and missing debounce on rapid-fire user actions.
- **UNI-008** (instrumentation balance): Look for `Err` branches with no `log::error!` or `log::warn!`. Flag `log::debug!` or `log::info!` inside loops over collection items. Check for PII in log interpolations.
- **UNI-009** (handle-then-throw): Look for `Err(e) => { model.field = ...; return Err(e) }` patterns where the model mutation is visible to the view but the error also propagates, leaving the UI in an inconsistent state.
- **UNI-011** (timeout/retry): Check whether effect handlers account for external calls that may hang or fail transiently. In the Crux core, this surfaces as missing timeout events or retry commands.
- **UNI-012** (persisted state compat): Check whether `PersistedState` struct changes include `#[serde(default)]` on new fields and whether removed fields use `#[serde(skip)]` or migration logic.
- **UNI-013** (dead code): Look for match arms shadowed by earlier guards, functions with no call sites, and Event variants never dispatched by any view.
- **UNI-014** (hardcoded config): Look for magic-number timeouts, hardcoded URL strings, and literal page sizes or retry counts.
- **UNI-015** (stale captures): Look for `Command` chains that capture model field values before an async operation and use the snapshot after resolution, when the model may have been mutated by intervening events.
- **UNI-016** (error message quality): Look for error messages with no item IDs, field names, or operation context.
- **UNI-017** (type safety): Beyond CRX-008 (ViewModel), look for `String` fields on model types, Event payloads, or PendingOp variants that hold values from a known closed set (should be enums or newtypes).
- **UNI-019** (injection vulnerabilities): Crux cores do not access databases or spawn processes directly (these go through effects), but check for user input interpolated into URL path segments, query strings, or HTML/XML output built as strings. Also check for `format!` used to construct structured data (JSON, SQL, URLs) rather than proper builders.
- **UNI-020** (unsafe deserialization): Look for deserialization of untrusted external payloads (SSE events, HTTP responses) directly into internal model types that carry authorization or privilege state. Check for missing size limits on payloads deserialized from effects.
- **UNI-021** (missing auth checks): In a Crux core, authentication is typically managed by the shell and passed as model state. Check that handlers for sensitive operations (delete, admin actions) verify `model.auth_state` or equivalent before proceeding. Flag handlers that assume authentication without checking.

Prefix findings from this step with `UNI-` occurrence IDs (e.g., `UNI-1`, `UNI-2`) and include the matching stable `rule_id` (e.g., `UNI-016`) on each finding. Use the severity defined by the rule.

Tag findings that have a **Spec-change indicator** (UNI-002, UNI-004, UNI-007, UNI-008, UNI-011, UNI-012, UNI-014, UNI-021) for inclusion in the adversarial review and spec-change output in step 3.
