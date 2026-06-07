# Review Categories

> **When to read this**: Read this when running a review pass to look up the full library of check categories. The Critical Path in `SKILL.md` already names the categories; this file enumerates each finding-id prefix, severity, the patterns each reviewer scans for, and the universal-check (UNI-) heuristics applied by the lead.

The reviewer team divides work across four finding-ID prefixes:

- `SEC-` — Security Reviewer (Security + WASM Constraints)
- `COR-` — Correctness Reviewer (Error Handling + Validation Logic + Provider Misuse)
- `QUA-` — Quality Reviewer (Performance + Code Quality)
- `UNI-` — Lead's universal-checks pass (gaps not covered by SEC/COR/QUA)

These prefixes are **report-local occurrence ids** — the `id` field on a structured `LintFinding` (the `LintFinding` schema uses the equivalent `FIND-0001` shape; this report uses prefixed counters for human triage). They restart in each report (`SEC-1`, `COR-1`, `UNI-1`) and must not be treated as stable codex ids. When a finding maps cleanly to a rule, add a separate `rule_id` field such as `OMNIA-002`, `RUST-001`, `SEC-001`, or `UNI-014` — three-digit codex ids matching `^(UNI|SRC|FRAME|RUST|IFACE|SEC|OMNIA|VECTIS|ORG)-[0-9]{3}$`. The markdown `rule_id:` prose maps to the kebab-case `rule-id` field on the `LintFinding` wire shape. Do not replace the occurrence id with the codex id.

Stable codex sources for this reviewer:

- [`adapters/targets/omnia/rules/`](../rules/) — Omnia-specific rules: `OMNIA-001` Provider-Only Host Access, `OMNIA-002` WASM Guest Runtime Constraints, `RUST-001` Classified SDK Errors, No Panic Paths, and `SEC-001` Host-Managed Secrets and Identity.
- [`adapters/shared/rules/universal/`](../../../shared/rules/universal/) — shared `UNI-001` through `UNI-021` rules. Read these codex files directly.

Prefer the most specific matching rule. For example, direct `std::env` access for a secret maps to `SEC-001`; direct `std::env` access for ordinary configuration maps to `OMNIA-002` or `OMNIA-001` depending on whether the core violation is WASM runtime behavior or provider bypass.

## Specialist categories

### 1. Security (critical)

Issues that could lead to data breaches, unauthorized access, or system compromise.

**Check for**:

- SQL injection vulnerabilities (`rule_id: UNI-019`)
- Command injection (shell execution with user input) (`rule_id: UNI-019`)
- XSS in HTML/XML output (`rule_id: UNI-019`)
- Path traversal vulnerabilities (`rule_id: UNI-002` when caused by missing input validation)
- Hardcoded secrets or credentials (`rule_id: SEC-001`, or `UNI-018` for non-Omnia generic secret findings)
- Unsafe deserialization (`rule_id: UNI-020`)
- Missing authentication checks (`rule_id: UNI-021`)

**Severity**: critical (must fix before deployment)

### 2. Error Handling (critical)

Missing error handling leads to panics and service outages.

**Check for**:

- `unwrap()` or `expect()` calls in production code (`rule_id: RUST-001`)
- Unhandled `Option::None` cases (`rule_id: RUST-001`)
- Unhandled `Result::Err` cases (`rule_id: RUST-001`)
- Errors that aren't propagated with `?` (`rule_id: RUST-001`)
- Generic error messages or unclassified SDK errors (`rule_id: RUST-001`, with `UNI-016` for generic message quality outside SDK classification)
- Swallowed errors (caught but not logged or returned) (`rule_id: RUST-001`)

**Severity**: critical (causes runtime panics)

### 3. WASM Constraints (critical)

Violations prevent compilation or cause runtime errors in WASM.

**Check for**:

- `std::env` usage (must use Config provider) (`rule_id: OMNIA-002`; use `SEC-001` when reading secrets)
- `std::fs` usage (must use `StateStore` for key-value state, `Blobstore` for binary files, `DocumentStore` for JSON documents, or `HttpRequest` for remote resources) (`rule_id: OMNIA-002`)
- `std::net` usage (must use HttpRequest provider) (`rule_id: OMNIA-002`)
- `std::thread` usage (must be async) (`rule_id: OMNIA-002`)
- Mutable global state (`static mut`, `OnceCell` outside `LazyLock` pattern) (`rule_id: OMNIA-002`)
- `unsafe` code blocks (`rule_id: OMNIA-002`)
- Direct blob/document client crates (`mongodb`, `azure_storage_blobs`, `aws-sdk-s3`) -- must use Blobstore/DocumentStore provider (`rule_id: OMNIA-001`)
- Blocking operations (synchronous I/O) (`rule_id: OMNIA-002`)

**Severity**: critical (build failure or runtime crash)

### 4. Provider Misuse (important)

Incorrect use of Omnia SDK providers.

**Check for**:

- Missing provider trait bounds on handlers (`rule_id: OMNIA-001`)
- Direct system calls instead of providers (`rule_id: OMNIA-001`)
- Provider methods called incorrectly (`rule_id: OMNIA-001`)
- Missing error handling on provider calls (`rule_id: RUST-001`)

**Severity**: important (functional bugs)

### 5. Validation Logic (important)

Missing or misplaced validation causes incorrect behavior.

**Check for**:

- No validation on required fields (`rule_id: UNI-002`)
- Structural validation in `handle()` instead of `from_input()` (`rule_id: UNI-002`)
- Temporal validation in `from_input()` instead of `handle()` (`rule_id: UNI-004`)
- Missing format validation (email, URL, phone) (`rule_id: UNI-002`)
- Missing range checks (amount > 0, length <= 1000) (`rule_id: UNI-002`)
- No business rule validation (`rule_id: UNI-004`)

**Severity**: important (accepts invalid data)

### 6. Performance (suggestion)

Inefficient patterns that cause slow response times.

**Check for**:

- N+1 query patterns (loop with API calls) (`rule_id: UNI-007`)
- Excessive HTTP requests (not batched) (`rule_id: UNI-007`)
- Missing caching for repeated data (`rule_id: UNI-005` when growth is unbounded, otherwise omit `rule_id` unless the universal codex has a clearer match)
- Large allocations in hot paths (`rule_id: UNI-005` when unbounded)
- Unnecessary cloning
- Synchronous operations in async context (`rule_id: OMNIA-002`)

**Severity**: suggestion (performance degradation)

### 7. Code Quality (optional)

Readability and maintainability issues.

**Check for**:

- Unclear variable names (`data`, `tmp`, `x`, `result`)
- Functions > 50 lines (consider splitting)
- Missing documentation for complex logic
- Inconsistent naming (snake_case violations)
- Dead code or unused variables (`rule_id: UNI-013`)
- Magic numbers (should be named constants) (`rule_id: UNI-014` when they are configuration values)

**Severity**: optional (technical debt)

## Universal checks (`UNI-` prefix)

After all three specialists report, the lead applies every `UNI-*` rule from [`adapters/shared/rules/universal/`](../../../shared/rules/universal/) with Omnia/WASM-specific detection. Read the first-party codex files directly. Several universal checks overlap with categories already assigned to the specialists. Skip those and focus on the gaps:

| Universal check                    | Already covered by                           | Action                |
| ---------------------------------- | -------------------------------------------- | --------------------- |
| UNI-002 Unvalidated input          | Validation Logic (COR)                       | Skip                  |
| UNI-003 Serialization failures     | Error Handling (COR)                         | Skip                  |
| UNI-006 Race conditions            | WASM Constraints (SEC) -- no threads in WASM | Skip                  |
| UNI-010 Panics/crashes             | Error Handling: unwrap/expect (COR)          | Skip                  |
| UNI-013 Dead code                  | Code Quality (QUA)                           | Skip                  |
| UNI-014 Hardcoded config (partial) | Provider Misuse: std::env (COR)              | Apply beyond env vars |
| UNI-018 Hardcoded secrets          | Security: hardcoded secrets (SEC)            | Skip                  |
| UNI-019 Injection vulnerabilities  | Security: SQL/command/XSS injection (SEC)    | Skip                  |
| UNI-020 Unsafe deserialization     | Security: unsafe deserialization (SEC)       | Skip                  |
| UNI-021 Missing auth checks        | Security: missing authentication (SEC)       | Skip                  |

Apply the remaining checks with these Omnia/WASM-specific heuristics:

- **UNI-001** (uninitialised values): Look for `#[derive(Default)]` on request or response structs where the default value has no valid domain meaning. Check `Option::None` fields used in handler logic without distinguishing "not provided" from "intentionally empty".
- **UNI-004** (logic bugs): Reason about handler control flow for inverted conditions, off-by-one errors in pagination or batch processing, and match arms that are always true or always false. Check `from_input()` for conditions that silently accept invalid data.
- **UNI-005** (unbounded growth): Look for `Vec` or `HashMap` fields built up inside handler functions without size limits. Check for loops that accumulate results from paginated API calls without a maximum page guard.
- **UNI-007** (chatty calls): Look for duplicate `HttpRequest::fetch` calls fetching the same URL within a single handler invocation. Check for handlers that re-fetch data obtainable from the request payload or from a prior call in the same flow.
- **UNI-008** (instrumentation balance): Look for `Err` branches with no `tracing::error!` or `tracing::warn!`. Flag `tracing::debug!` or `tracing::info!` inside loops over collection items. Check for PII (names, emails, tokens) interpolated into tracing spans.
- **UNI-009** (handle-then-throw): Look for error paths that mutate provider-backed state (`StateStore::set`, `Publish::send`) before returning an error, leaving the external system in an inconsistent state while the caller sees a failure.
- **UNI-011** (timeout/retry): Check whether `HttpRequest::fetch` calls account for upstream timeouts or transient failures. Flag handlers that have no retry or fallback path for external calls that may hang.
- **UNI-012** (persisted state compat): Check whether `StateStore` value types that changed (new fields, renamed fields, changed types) include `#[serde(default)]` on new fields or migration logic for existing keys.
- **UNI-014** (hardcoded config, beyond env vars): Look for magic-number timeouts, literal URL path segments, hardcoded retry counts, and page sizes embedded in handler code rather than sourced from `Config::get`.
- **UNI-015** (stale captures): Look for async blocks that capture local variables which are mutated between the capture point and the `.await` resumption. Check for closures passed to iterator combinators that capture mutable references.
- **UNI-016** (error message quality): Look for `bad_request!` or `server_error!` calls with generic messages ("invalid input", "failed") that omit the field name, value, or operation that caused the failure.
- **UNI-017** (type safety): Look for `String` fields on request/response types that hold values from a known closed set (should be enums). Check for ID fields typed as plain `String` that are interchangeable with unrelated IDs (should be newtypes per Omnia strong-typing conventions).

Prefix findings from this step with `UNI-` (e.g., UNI-1, UNI-2). Use the severity defined in the universal checklist for each check.

For each universal finding, also set `rule_id` to the stable codex ID that triggered it (for example, local finding `UNI-2` may carry `rule_id: UNI-014`). Use the severity from the rule.

Tag findings that have a **Spec-change indicator** (`UNI-002`, `UNI-004`, `UNI-007`, `UNI-008`, `UNI-011`, `UNI-012`, `UNI-014`, `UNI-021`) for inclusion in the Adversarial Review and report synthesis. When the spec is silent on the concern a check raises, surface the finding as a `spec-change` design finding for the operator's `/spec:plan` follow-up rather than auto-spawning a Specify slice.
