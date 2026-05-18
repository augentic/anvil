# Universal Codex Checks — Android/Compose Heuristics

Read this at step 2c of the review-fix cycle, after the specialists complete and before the antagonist runs.

The lead applies universal codex rules `UNI-001` through `UNI-021` from the resolved default codex (`adapters/default/codex/*.md`). Several universal checks overlap with categories already covered by the specialists; skip those:

| Universal check | Already covered by | Action |
|---|---|---|
| UNI-003 Serialization failures | AND-013, AND-014, AND-020 | Skip |
| UNI-006 Race conditions | KTL-003, AND-015, AND-016 | Skip |
| UNI-010 Panics/crashes | KTL-001 | Skip |

Apply the remaining checks with these Kotlin/Android-specific heuristics:

- **UNI-001** (uninitialised values): Look for `var` properties initialised to `null` or placeholder values that are accessed before a coroutine load completes. Check for `MutableStateFlow` initialised with default values that represent an invalid domain state.
- **UNI-002** (unvalidated input): Look for shell-side `TextField` values dispatched to the core via `onEvent(Event.Something(text))` without local trim or empty check. While the core should also validate, the shell should prevent obviously invalid dispatches.
- **UNI-004** (logic bugs): Reason about the `processRequest` `when` for missing branches, incorrect effect resolution sequences, and navigation handlers that produce unreachable states.
- **UNI-005** (unbounded growth): Look for `scope.launch` blocks that create coroutines without cancellation tracking, growing lists of SSE observations without cleanup, and `MutableStateFlow` subscribers that are never collected. Check for `Job` references stored without cancellation.
- **UNI-007** (chatty calls): Look for Ktor HTTP calls that re-fetch data the core already has from SSE or other real-time channels. Check for effect handlers that fire identical resolve calls on repeated recompositions.
- **UNI-008** (instrumentation balance): Look for error paths with no `Log.e` or `Log.w` call. Flag per-event logging inside hot loops (e.g., logging every SSE chunk body).
- **UNI-009** (handle-then-throw): Look for `try/catch` blocks that partially update `_viewModel.value` or other `MutableStateFlow` values before rethrowing, leaving the UI in an inconsistent state.
- **UNI-011** (timeout/retry): Look for Ktor `HttpClient` instances without `HttpTimeout` installed. Check whether SSE reconnection logic exists for transient network failures.
- **UNI-012** (persisted state compat): Look for `SharedPreferences` model changes (new keys, changed serialization format) that would break deserialization of existing stored data.
- **UNI-013** (dead code): Look for `when` branches that can never match, unreachable code after `return` / `break`, unused private functions or properties, and composables with no call site.
- **UNI-014** (hardcoded config): Look for hardcoded timeout intervals, literal URL strings, and magic number page sizes or retry counts.
- **UNI-015** (stale captures): Look for `scope.launch` blocks capturing `this` or local state that may mutate before the coroutine completes. Check for lambda captures in `LazyColumn` `items` blocks that reference loop-scoped variables.
- **UNI-016** (error message quality): Look for `Log.e` messages with no context about which item or operation failed, and catch blocks that log the exception type but not the message.
- **UNI-017** (type safety): Look for `String` properties on view model types or event types that hold values from a known closed set (should be Kotlin enums or sealed interfaces).
- **UNI-018** (hardcoded secrets): Look for API keys, tokens, passwords, or connection strings embedded as string literals in Kotlin source files. Check for secrets in `local.properties` committed to git, hardcoded `Authorization` headers, and credentials stored in plain-text `SharedPreferences` rather than `EncryptedSharedPreferences` or the Android Keystore.
- **UNI-019** (injection vulnerabilities): Look for user input interpolated into `WebView` HTML content without escaping, URL path segments built via string concatenation, and `Runtime.exec` invocations with user-controlled arguments.
- **UNI-020** (unsafe deserialization): Look for bincode or JSON deserialization of untrusted external payloads directly into model types that carry privilege state. Check for missing payload size limits on data fetched from external sources.
- **UNI-021** (missing auth checks): Check that effect handlers attaching authentication credentials (Bearer tokens, API keys) to outbound requests source them from secure storage (Android Keystore / `EncryptedSharedPreferences`), not from hardcoded values or unprotected `SharedPreferences`. Flag API calls to protected endpoints dispatched without any auth header.

Prefix findings from this step with `UNI-` occurrence IDs (e.g., `UNI-1`, `UNI-2`) and include the matching stable `rule_id` (e.g., `UNI-016`) on each finding. Use the severity defined by the codex rule.

Tag findings that have a **Spec-change indicator** (UNI-002, UNI-004, UNI-007, UNI-008, UNI-011, UNI-012, UNI-014, UNI-021) for inclusion in the adversarial review and spec-change output in step 3.
