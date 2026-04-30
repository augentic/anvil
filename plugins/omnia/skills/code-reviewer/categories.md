# Review Categories

> **When to read this**: Read this when running a review pass to look up the full library of check categories. The Critical Path in `SKILL.md` already names the categories; this file enumerates each finding-id prefix, severity, the patterns each reviewer scans for, and the universal-check (UNI-) heuristics applied by the lead.

The reviewer team divides work across four finding-ID prefixes:

- `SEC-` — Security Reviewer (Security + WASM Constraints)
- `COR-` — Correctness Reviewer (Error Handling + Validation Logic + Provider Misuse)
- `QUA-` — Quality Reviewer (Performance + Code Quality)
- `UNI-` — Lead's universal-checks pass (gaps not covered by SEC/COR/QUA)

## Specialist categories

### 1. Security (CRITICAL)

Issues that could lead to data breaches, unauthorized access, or system compromise.

**Check for**:

- SQL injection vulnerabilities
- Command injection (shell execution with user input)
- XSS in HTML/XML output
- Path traversal vulnerabilities
- Hardcoded secrets or credentials
- Unsafe deserialization
- Missing authentication checks

**Severity**: CRITICAL (must fix before deployment)

### 2. Error Handling (CRITICAL)

Missing error handling leads to panics and service outages.

**Check for**:

- `unwrap()` or `expect()` calls in production code
- Unhandled `Option::None` cases
- Unhandled `Result::Err` cases
- Errors that aren't propagated with `?`
- Generic error messages (no context)
- Swallowed errors (caught but not logged)

**Severity**: CRITICAL (causes runtime panics)

### 3. WASM Constraints (CRITICAL)

Violations prevent compilation or cause runtime errors in WASM.

**Check for**:

- `std::env` usage (must use Config provider)
- `std::fs` usage (must use `StateStore` for key-value state, `Blobstore` for binary files, `DocumentStore` for JSON documents, or `HttpRequest` for remote resources)
- `std::net` usage (must use HttpRequest provider)
- `std::thread` usage (must be async)
- Mutable global state (`static mut`, `OnceCell` outside `LazyLock` pattern)
- `unsafe` code blocks
- Direct blob/document client crates (`mongodb`, `azure_storage_blobs`, `aws-sdk-s3`) -- must use Blobstore/DocumentStore provider
- Blocking operations (synchronous I/O)

**Severity**: CRITICAL (build failure or runtime crash)

### 4. Provider Misuse (HIGH)

Incorrect use of Omnia SDK providers.

**Check for**:

- Missing provider trait bounds on handlers
- Direct system calls instead of providers
- Provider methods called incorrectly
- Missing error handling on provider calls

**Severity**: HIGH (functional bugs)

### 5. Validation Logic (HIGH)

Missing or misplaced validation causes incorrect behavior.

**Check for**:

- No validation on required fields
- Structural validation in `handle()` instead of `from_input()`
- Temporal validation in `from_input()` instead of `handle()`
- Missing format validation (email, URL, phone)
- Missing range checks (amount > 0, length <= 1000)
- No business rule validation

**Severity**: HIGH (accepts invalid data)

### 6. Performance (MEDIUM)

Inefficient patterns that cause slow response times.

**Check for**:

- N+1 query patterns (loop with API calls)
- Excessive HTTP requests (not batched)
- Missing caching for repeated data
- Large allocations in hot paths
- Unnecessary cloning
- Synchronous operations in async context

**Severity**: MEDIUM (performance degradation)

### 7. Code Quality (LOW)

Readability and maintainability issues.

**Check for**:

- Unclear variable names (`data`, `tmp`, `x`, `result`)
- Functions > 50 lines (consider splitting)
- Missing documentation for complex logic
- Inconsistent naming (snake_case violations)
- Dead code or unused variables
- Magic numbers (should be named constants)

**Severity**: LOW (technical debt)

## Universal checks (`UNI-` prefix)

After all three specialists report, the lead reads `references/review-checks.md` and applies checks UNI-001 through UNI-021 with Omnia/WASM-specific detection. Several universal checks overlap with categories already assigned to the specialists. Skip those and focus on the gaps:

| Universal check | Already covered by | Action |
|---|---|---|
| UNI-002 Unvalidated input | Validation Logic (COR) | Skip |
| UNI-003 Serialization failures | Error Handling (COR) | Skip |
| UNI-006 Race conditions | WASM Constraints (SEC) -- no threads in WASM | Skip |
| UNI-010 Panics/crashes | Error Handling: unwrap/expect (COR) | Skip |
| UNI-013 Dead code | Code Quality (QUA) | Skip |
| UNI-014 Hardcoded config (partial) | Provider Misuse: std::env (COR) | Apply beyond env vars |
| UNI-018 Hardcoded secrets | Security: hardcoded secrets (SEC) | Skip |
| UNI-019 Injection vulnerabilities | Security: SQL/command/XSS injection (SEC) | Skip |
| UNI-020 Unsafe deserialization | Security: unsafe deserialization (SEC) | Skip |
| UNI-021 Missing auth checks | Security: missing authentication (SEC) | Skip |

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

Tag findings that have a **Spec-change indicator** (UNI-002, UNI-004, UNI-007, UNI-008, UNI-011, UNI-012, UNI-014, UNI-021) for inclusion in the Adversarial Review and report synthesis. When the spec is silent on the concern a check raises, note the finding as a candidate for a spec update via `/spec:define`.
