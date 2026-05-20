---
name: omnia-code-reviewer
description: Review generated Omnia Rust WASM crates for security, error handling, WASM constraints, and code quality issues. Use when `crate-writer` has just produced or updated an Omnia crate and the slice is ready for review; not for reviewing tests (`test-writer` covers its own loop) or guest wrappers.
argument-hint: "[crate-path]"
---

# AI Code Review Skill

## Critical Path

1. **Parse invocation**: resolve `$CRATE_PATH` and the `fix` positional; verify `cargo check` passes (see [Invocation](#invocation)).
2. **Initialize team**: spawn Security, Correctness, and Quality specialist sub-agents with the prompts in [`team-protocol.md`](team-protocol.md).
3. **Specialist analysis (concurrent)**: each specialist scans `src/*.rs` and emits findings prefixed `SEC-`, `COR-`, `QUA-` per [`categories.md`](categories.md), adding `rule_id` when a finding maps to a stable codex rule.
4. **Universal checks (lead)**: lead applies default codex rules UNI-001…UNI-021 with the Omnia/WASM heuristics in [`categories.md`](categories.md#universal-checks-uni--prefix), prefixing report-local findings `UNI-` and setting `rule_id` to the matching stable codex ID.
5. **Adversarial challenge**: lead forwards all findings to the antagonist, which confirms / upgrades / downgrades / disputes them and adds `NEW-` findings (rules in [`team-protocol.md`](team-protocol.md)).
6. **Synthesis**: lead writes `$REVIEW_OUTPUT` using the template in [`output.md`](output.md), recording adversarial-review statistics and a confidence level.
7. **Auto-fix (only if `fix`)**: lead applies safe fixes per [`auto-fix.md`](auto-fix.md), runs `cargo check`, and reverts on failure. Then shut down the team.

## Overview

Perform comprehensive AI-powered code review on generated Rust WASM crates, identifying security vulnerabilities, missing validation, performance issues, and code quality problems. The skill provides automated detection of common AI code issues, specific fixes for critical problems (not just "check this"), an auto-fix path for simple issues (`fix`), and educational feedback to improve future generations.

## Invocation

The argument-hint advertises a single positional `crate-path`. The `fix` positional is parsed from the raw arguments inside the body — it is intentionally **not** in the hint so the slash-command UI stays single-token.

```text
$CRATE_PATH    = $ARGUMENTS[0]
$AUTO_FIX      = "fix" in $ARGUMENTS  # boolean
$REVIEW_OUTPUT = $CRATE_PATH/REVIEW.md
```

**Positional arguments**:

- `fix` (optional) — apply auto-fixes for confirmed/upgraded auto-fixable findings after synthesis. See [`auto-fix.md`](auto-fix.md) for the full scope, success-rate table, and regression guard.

**Prerequisites**:

- Generated Rust crate (from `crate-writer`).
- Crate must compile (`cargo check` passes) before review starts; the auto-fix step re-runs `cargo check` after applying fixes.

## Review pipeline

The skill drives an agent team — three specialist reviewers plus an antagonist — coordinated by the lead. The pipeline implements the seven-step Critical Path above:

1. The lead spawns the three specialists concurrently with the prompts in [`team-protocol.md`](team-protocol.md). Each specialist owns a slice of the categories enumerated in [`categories.md`](categories.md).
2. Once all specialists report, the lead runs the **universal checks** pass over `src/`, using the default codex (`adapters/default/codex/`) for `UNI-001` through `UNI-021` and applying only the checks not already covered by SEC/COR/QUA per the skip table in [`categories.md`](categories.md#universal-checks-uni--prefix).
3. The lead forwards the combined findings (`SEC-`, `COR-`, `QUA-`, `UNI-`) to the antagonist. The antagonist confirms, upgrades, downgrades, or disputes each finding and runs a counter-scan that may emit `NEW-` findings.
4. The lead synthesizes the final report using the template in [`output.md`](output.md), assigns a confidence level via the [Agent Team Patterns](references/agent-teams.md#confidence-scoring), and writes `$REVIEW_OUTPUT`.
5. If `$AUTO_FIX == true`, the lead applies safe fixes per [`auto-fix.md`](auto-fix.md), then shuts down the team.

The four sibling files are the load-on-demand depth for this skill. Read them only at the moments named in the Critical Path; do **not** load them eagerly.

## Output shape

The review report lives at `$CRATE_PATH/REVIEW.md`. The full template — including finding-ID prefix conventions, severity glyphs, the **Adversarial Review** section, the **Auto-Fix Summary** block, and the **Quality Metrics** comparison table — is in [`output.md`](output.md).

Key invariants the lead must preserve:

- Every finding has a `file:line` reference and a code snippet (verbatim from the originating reviewer).
- Every mapped finding carries both a report-local occurrence ID (`SEC-1`, `COR-1`, `QUA-1`, `UNI-1`, `NEW-1`) and a separate stable `rule_id` such as `OMNIA-002` or `UNI-014`.
- Severity reflects antagonist adjustments — upgrades and downgrades rewrite the displayed severity but keep the original prefix and ID.
- Finding IDs use the prefixes documented in [`output.md`](output.md): `SEC-`, `COR-`, `QUA-`, `UNI-`, `NEW-`.
- `rule_id` is a codex citation only. Do not claim or assume the final RM-04 finding schema exists.
- The **Adversarial Review** section reports challenge statistics (confirmed / downgraded / upgraded / disputed / new) and the acceptance rate.
- Auto-fix outcomes (✅ applied, ⚠️ reverted, ⏭️ skipped) appear inline on each finding and aggregated in the **Auto-Fix Summary**.

## Reference documentation

- [`categories.md`](categories.md) — Full SEC-/COR-/QUA-/UNI- check libraries, Omnia/WASM heuristics, and codex `rule_id` mapping guidance.
- [`team-protocol.md`](team-protocol.md) — Specialist spawn prompts, antagonist protocol, synthesis rules.
- [`auto-fix.md`](auto-fix.md) — `fix` scope, success-rate table, regression guard, recovery process.
- [`output.md`](output.md) — `REVIEW.md` template and finding-ID conventions.
- [Default Codex](../../../../adapters/default/codex/) — Source of truth for universal rules `UNI-001`…`UNI-021`.
- [Omnia Codex](../../../../adapters/omnia/codex/) — Source of truth for Omnia-specific rules `OMNIA-001`, `OMNIA-002`, `RUST-001`, and `SEC-001`.
- [Agent Team Patterns](references/agent-teams.md) — Shared team roles, antagonist protocol, synthesis rules, and file ownership.
- [CodeRabbit Study: AI Code Creates 1.7× More Issues](https://www.coderabbit.ai/blog/state-of-ai-vs-human-code-generation-report)
- [Security Best Practices for Rust](https://anssi-fr.github.io/rust-guide/)
- [WASM Security Model](https://webassembly.org/docs/security/)

## Examples

### Example 1: Simple CRUD review

**Input**: `crate-path` pointing to a generated order management crate.

**Review finds**:

- Critical: `unwrap()` on user-provided customer ID lookup
- High: missing input length validation on `description` field
- Medium: N+1 HTTP calls in order listing endpoint

**Auto-fix applies**: replaces `unwrap()` with explicit Omnia-compatible errors (for example `bad_request!` or `Error::NotFound { code, description }`). Remaining issues documented in `REVIEW.md`.

### Example 2: Complex workflow review

**Input**: `crate-path` pointing to a payment processing crate.

**Review finds**:

- Critical: hardcoded API key in test fixture (should use Config provider)
- Critical: missing error propagation on HTTP timeout
- High: `std::thread::sleep` used instead of `tokio::time::sleep`

**Auto-fix applies**: replaces `std::thread::sleep` with the async equivalent and adds `?` for error propagation. The hardcoded key is flagged for manual fix.

## Verification checklist

Before completing review:

### Team execution

- [ ] All 3 specialists spawned with correct category assignments (see [`team-protocol.md`](team-protocol.md))
- [ ] All specialists completed before antagonist spawned
- [ ] Antagonist received all specialist findings
- [ ] Antagonist provided evidence for every challenge
- [ ] Lead synthesized all findings into `REVIEW.md`
- [ ] Team shut down and cleaned up

### Scan coverage

- [ ] Security Reviewer: SQL injection, XSS, secrets, WASM constraints checked
- [ ] Correctness Reviewer: unwrap/expect, validation placement, provider usage checked
- [ ] Quality Reviewer: N+1 patterns, naming, function length, dead code checked
- [ ] Universal checks: UNI-001…UNI-021 applied with Omnia heuristics (skipped where covered by SEC/COR/QUA)
- [ ] Antagonist: counter-scan completed for blind spots
- [ ] Mapped findings include stable codex `rule_id` values without changing occurrence IDs

### Report quality

- [ ] Each issue has `file:line` reference and code snippet
- [ ] Severity reflects antagonist adjustments (upgrades/downgrades applied)
- [ ] **Adversarial Review** section included with challenge statistics
- [ ] Confidence level assigned based on antagonist results
- [ ] Finding IDs use correct prefixes (`SEC-`, `COR-`, `QUA-`, `UNI-`, `NEW-`)
- [ ] `rule_id` values cite existing codex rules only; unmapped findings do not invent IDs

### Auto-fix (if enabled)

- [ ] Auto-fix gates from [`auto-fix.md`](auto-fix.md) satisfied (confirmed/upgraded only, antagonist regression flags respected, `cargo check` passed, revert on failure)

## Expected results

### Typical issue counts by crate complexity

**Simple CRUD** (200-300 LOC): Critical 0-2 · High 2-5 · Medium 1-3 · Low 5-10
**Business Logic** (500-800 LOC): Critical 2-5 · High 5-10 · Medium 3-6 · Low 10-20
**Complex Workflows** (1000+ LOC): Critical 5-10 · High 10-20 · Medium 5-10 · Low 20-40

The auto-fix success-rate table per category lives in [`auto-fix.md`](auto-fix.md#auto-fix-success-rate-per-category).

## Integration with `/spec:build`

Add code review near the end of implementation, after generation and verification:

```bash
/code-reviewer $CRATE_PATH fix

if grep "Critical Issues: [1-9]" $CRATE_PATH/REVIEW.md; then
    echo "Critical issues found - manual review required"
    echo "See $CRATE_PATH/REVIEW.md for details"
fi
```
