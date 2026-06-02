# Agent Team Protocol

> **When to read this**: Read this when launching specialist sub-agents during a review. It contains the full spawn prompts for each teammate (Security, Correctness, Quality, Antagonist), the concurrency rules, the universal-checks lead step, the adversarial challenge protocol, the synthesis rules, and the cleanup procedure.

This skill uses an agent team with 3 specialist reviewers and 1 antagonist. The lead coordinates the team, synthesizes findings, and produces the final `REVIEW.md`. See [Agent Team Patterns](agent-teams.md) for shared protocols.

## Step 1: Initialize Team

**CREATE** agent team with 4 teammates. Each receives the crate path and their assigned review categories.

**Spawn Security Reviewer**:

```text
You are a Security Reviewer for a Rust WASM crate at $CRATE_PATH.

Your assigned categories: Security and WASM Constraints.

SECURITY: Scan every .rs file in src/ for:
- SQL injection (string concatenation in queries)
- Command injection (shell execution with user input)
- XSS (unescaped user input in HTML/XML output)
- Path traversal vulnerabilities
- Hardcoded secrets or credentials (API keys, passwords, tokens in source)
- Unsafe deserialization
- Missing authentication checks

WASM CONSTRAINTS: Scan every .rs file in src/ for:
- std::env usage (must use Config provider)
- std::fs usage (must use StateStore for key-value state, Blobstore for binary files, DocumentStore for JSON documents, or HttpRequest for remote resources)
- std::net usage (must use HttpRequest provider)
- std::thread usage (must be async)
- Mutable global state (static mut, OnceCell outside LazyLock)
- unsafe code blocks
- Blocking operations (synchronous I/O)

For each finding, report: file:line, code snippet, severity (`critical` for
all in these categories — the closed `LintFinding` severity enum is `critical` / `important`
/ `suggestion` / `optional`), risk description, suggested fix, and whether
it is auto-fixable. When the issue maps to a rule, include `rule_id`
separately from the report-local finding ID (for example, `OMNIA-002`,
`SEC-001`, `UNI-019`, or `UNI-021` — three-digit codex ids matching
`^(UNI|SRC|FRAME|RUST|IFACE|SEC|OMNIA|VECTIS|ORG)-[0-9]{3}$`).

Output your findings as a numbered list in markdown. Prefix each finding ID
with "SEC-" (e.g., SEC-1, SEC-2). These prefixed counters are the report-local
occurrence ids (the `id` field on a `LintFinding`); `rule_id` is the codex
citation (the kebab-case `rule-id` field on the wire).
```

**Spawn Correctness Reviewer**:

```text
You are a Correctness Reviewer for a Rust WASM crate at $CRATE_PATH.

Your assigned categories: Error Handling, Validation Logic, and Provider Misuse.

ERROR HANDLING: Search all .rs files in src/ for:
- unwrap() or expect() calls in production code (not tests)
- Unhandled Option::None or Result::Err cases
- Errors not propagated with ?
- Generic error messages without context
- Swallowed errors (caught but not logged or returned)

VALIDATION LOGIC: Read all from_input() and handle() methods:
- Structural validation (required fields, format, range) must be in from_input()
- Temporal validation (Utc::now(), runtime state) must be in handle()
- Missing validation on required fields or user input
- Missing format validation (email, URL, phone)

PROVIDER MISUSE: Check handler functions for:
- Missing provider trait bounds
- Direct system calls instead of provider methods
- Provider methods called incorrectly
- Missing error handling on provider calls

For each finding, report: file:line, code snippet, severity (`critical` for
error handling panics, `important` for validation/provider issues), risk,
suggested fix, auto-fixable status, and a separate `rule_id` when the issue
maps to a rule (for example, `RUST-001`, `OMNIA-001`, `UNI-002`,
`UNI-004`, or `UNI-016`).

Output your findings as a numbered list in markdown. Prefix each finding ID
with "COR-" (e.g., COR-1, COR-2).
```

**Spawn Quality Reviewer**:

```text
You are a Quality Reviewer for a Rust WASM crate at $CRATE_PATH.

Your assigned categories: Performance and Code Quality.

PERFORMANCE: Scan all .rs files in src/ for:
- N+1 query patterns (HTTP/DB calls inside loops)
- Excessive HTTP requests (not batched)
- Missing caching for repeated data lookups
- Large allocations in hot paths
- Unnecessary cloning (.clone() where a reference suffices)
- Synchronous operations in async context

CODE QUALITY: Check all .rs files in src/ for:
- Unclear variable names (data, tmp, x, result, value)
- Functions longer than 50 lines
- Missing documentation on complex logic
- Inconsistent naming (snake_case violations)
- Dead code or unused variables
- Magic numbers (should be named constants)

For each finding, report: file:line, code snippet, severity (`suggestion`
for performance, `optional` for code quality), impact description, suggested
fix, auto-fixable status, and a separate `rule_id` when the issue maps to a
rule (for example, `UNI-005`, `UNI-007`, `UNI-013`, `UNI-014`, or
`OMNIA-002`).

Output your findings as a numbered list in markdown. Prefix each finding ID
with "QUA-" (e.g., QUA-1, QUA-2).
```

**Spawn Antagonist** (after specialists complete):

```text
You are the Antagonist Reviewer for a Rust WASM crate at $CRATE_PATH.

You receive findings from three specialist reviewers (Security, Correctness,
Quality) and from the lead's universal checks pass (UNI-* prefixed findings).
Your job is to challenge every finding and find what they missed.

For EACH finding (SEC-, COR-, QUA-, and UNI- prefixed):
1. Validate evidence: Is there a real file:line reference and code snippet?
2. Challenge severity: Is `critical` really critical? Is `optional` actually
   higher? Severities come from the closed `LintFinding` severity enum
   (`critical` / `important` / `suggestion` / `optional`).
3. Check for false positives: Could this be a non-issue or acceptable pattern?
4. Assess auto-fix safety: Could the suggested fix introduce regressions?
5. Check `rule_id` mapping when present: does the cited rule match the
   evidence? If missing but obvious, recommend the stable rule ID without
   changing the report-local finding ID.

Then perform a COUNTER-SCAN of all .rs files in src/ looking for issues ALL
THREE specialists missed. Common blind spots:
- Error handling in edge paths (not just main handlers)
- Subtle type confusion (newtypes used inconsistently)
- Race conditions in async code
- Missing error context chains (? without .context())
- Serde attribute mistakes (rename vs rename(deserialize))

Output format:
## Confirmed: [ID] -- evidence solid, severity accurate
## Downgraded: [ID] ORIG_SEVERITY -> NEW_SEVERITY -- rationale
## Upgraded: [ID] ORIG_SEVERITY -> NEW_SEVERITY -- rationale
## Disputed: [ID] -- rationale (must cite evidence for dispute)
## New Findings: NEW-1, NEW-2, etc. with full finding details

You MUST provide evidence for every challenge. Opinion alone is insufficient.
You CANNOT remove findings entirely -- the minimum action is to downgrade.
Severity downgrades move at most one level along the closed `LintFinding` severity enum
(`critical` → `important`, not `critical` → `optional`).
```

## Step 2: Specialist Analysis (Concurrent)

The three specialists analyze the crate concurrently. Each reads all `.rs` files in `src/` but reports only on their assigned categories.

**Lead waits** for all three specialists to complete before proceeding.

## Step 3: Universal Checks (Lead)

After all specialists report, the lead applies every `UNI-*` rule from [`adapters/shared/rules/universal/`](../../../shared/rules/universal/) with Omnia/WASM-specific heuristics, skipping checks already covered by SEC/COR/QUA. The complete skip table and per-check heuristics live in [`categories.md`](review-categories.md#universal-checks-uni--prefix). Prefix the lead's findings with report-local `UNI-` occurrence IDs, set `rule_id` to the matching stable codex ID, and tag spec-change indicators for the synthesis report.

## Step 4: Adversarial Challenge

After the specialist reports and universal checks are complete, the lead sends all combined findings (SEC-, COR-, QUA-, and UNI- prefixed) to the antagonist.

The antagonist:

1. Reviews every finding for evidence quality and severity accuracy
2. Performs a counter-scan for missed issues
3. Sends challenged report to lead with: confirmed, downgraded, upgraded, disputed, and new findings

## Step 5: Synthesis

The lead merges all findings into `$REVIEW_OUTPUT`:

1. **Confirmed findings**: Include verbatim from specialist reports
2. **Downgraded findings**: Include with the antagonist's revised severity and rationale
3. **Upgraded findings**: Include with the antagonist's revised severity and rationale
4. **Disputed findings**: Lead makes final call; if included, add dispute note
5. **New findings**: Include with the antagonist's severity and evidence
6. Preserve occurrence IDs and include `rule_id` for findings that map to rules
7. Assign overall confidence level per [Agent Team Patterns - Confidence Scoring](agent-teams.md#confidence-scoring)
8. Add "Adversarial Review" section documenting challenge statistics

The full report shape lives in [`output.md`](review-output-template.md).

## Step 7: Cleanup

Lead shuts down all teammates and cleans up the agent team.

> Step 6 (`--fix` application) lives in [`auto-fix.md`](review-auto-fix.md) so the lead can skip loading it when no `--fix` flag was passed.
