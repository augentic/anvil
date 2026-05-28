# Review Output Template

> **When to read this**: Read this when emitting a `REVIEW.md` or finding output. It contains the full `REVIEW.md` template, finding-ID prefix conventions, and the adversarial-review / quality-metrics sections that the synthesis step writes verbatim.

## Finding-ID prefix conventions

Every finding in `REVIEW.md` carries a review-local prefix so the originating reviewer (or pass) is preserved through synthesis and auto-fix:

| Prefix | Origin                     | Categories                                        | Default severities                                                         |
| ------ | -------------------------- | ------------------------------------------------- | -------------------------------------------------------------------------- |
| `SEC-` | Security Reviewer          | Security, WASM Constraints                        | critical                                                                   |
| `COR-` | Correctness Reviewer       | Error Handling, Validation Logic, Provider Misuse | critical (errors) / important (validation, provider)                       |
| `QUA-` | Quality Reviewer           | Performance, Code Quality                         | suggestion (perf) / optional (quality)                                     |
| `UNI-` | Lead universal-checks pass | Gaps not covered by SEC/COR/QUA                   | Per [`adapters/shared/rules/universal/`](../../../shared/rules/universal/) |
| `NEW-` | Antagonist counter-scan    | Anything missed by the four passes above          | As supplied by antagonist                                                  |

Severity values use the closed `LintFinding` severity enum: `critical`, `important`, `suggestion`, `optional`. Numbering restarts at 1 within each prefix (`SEC-1`, `SEC-2`, …). When the antagonist upgrades or downgrades a finding, the original prefix is preserved and the severity change is recorded in the **Adversarial Review** section.

`SEC-1`, `COR-1`, `UNI-3` and similar are **report-local occurrence ids** — the `id` field on a structured `LintFinding` (the `LintFinding` schema uses the equivalent `FIND-0001` shape; this report uses prefixed counters for human triage). They are distinct from the codex `rule-id`. When a finding maps to a stable rule, add a separate `rule_id` line with the canonical three-digit value (e.g. `OMNIA-001`, `OMNIA-002`, `RUST-001`, `SEC-001`, or a `UNI-NNN` rule). Markdown reports render this as `rule_id:` or `Rule:` prose; the structured `LintFinding` wire shape uses the kebab-case `rule-id` field per the `LintFinding` schema. Leave the field out for genuinely unmapped findings; do not invent a rule ID.

## REVIEW.md template

````markdown
# Code Review Report

**Generated**: [timestamp]
**Crate**: [name]
**Review Team**: 3 specialists + 1 antagonist
**Auto-fix**: [enabled/disabled]
**Confidence Level**: [high | medium | low]

---

## Summary

- 🔴 critical: [count]
- 🟠 important: [count]
- 🟡 suggestion: [count]
- 🔵 optional: [count]

**Overall Assessment**: [Excellent | Good | Fair | Poor]

---

## 🔴 Critical (MUST FIX)

### SEC-1: WASM Constraint Violation

**File**: [src/config.rs:23](src/config.rs#L23)
**rule_id**: OMNIA-002
**Category**: WASM Compliance
**Reviewer**: Security Reviewer
**Antagonist**: ✅ Confirmed

**Issue**: Direct environment variable access (std::env)

\```rust
let api_url = std::env::var("API_URL").unwrap();
\```

**Risk**: Compilation failure or runtime panic in WASM
**Fix Applied**: ✅ Auto-fixed

\```rust
let api_url = ctx.config.get("API_URL")?;
\```

---

### COR-1: Missing Error Handling (Potential Panic)

**File**: [src/handlers.rs:67](src/handlers.rs#L67)
**rule_id**: RUST-001
**Category**: Error Handling
**Reviewer**: Correctness Reviewer
**Antagonist**: ⬆️ Upgraded from important to critical (untrusted input path)

[... finding details ...]

---

## 🟠 Important

[... important findings ...]

## 🟡 Suggestion

[... suggestion findings ...]

## 🔵 Optional

[... optional findings ...]

---

## Adversarial Review

**Antagonist Activity Summary**:

| Action       | Count   |
| ------------ | ------- |
| Confirmed    | [count] |
| Downgraded   | [count] |
| Upgraded     | [count] |
| Disputed     | [count] |
| New Findings | [count] |

**Acceptance Rate**: [confirmed / total specialist findings]%

### Downgraded Findings

- [COR-3] important → suggestion: Missing length validation on description field
  **Rationale**: Field is bounded by serde max_length attribute at deserialization

### Upgraded Findings

- [COR-1] important → critical: unwrap() on untrusted input path
  **Rationale**: Input comes directly from HTTP request body; attacker-controlled

### Disputed Findings

- [SEC-5] Reported as critical: "potential SQL injection"
  **Dispute**: No SQL database used; query passed to HttpRequest provider
  **Lead Decision**: Excluded (antagonist rationale accepted)

### New Findings (Missed by Specialists)

- [NEW-1] critical: Missing error propagation in retry loop (src/handlers.rs:112)
  **Evidence**: errors.push(e) swallows errors; Ok(()) returned when all retries fail

---

## Auto-Fix Summary

**Total Fixes Applied**: [count]
**Successful**: [count]
**Failed**: [count]

**Modified Files**:

- src/handlers.rs ([count] fixes)
- src/config.rs ([count] fixes)
- src/types.rs ([count] fixes)

**Verification**: [✅ cargo check passed | ⚠️ reverted due to errors]

---

## Quality Metrics

| Metric                   | This Crate | AI Baseline | Human Baseline | Status   |
| ------------------------ | ---------- | ----------- | -------------- | -------- |
| Issues per 100 LOC       | [n]        | 1.8         | 1.1            | [status] |
| Critical issues          | [n]        | 5 (est.)    | 2 (est.)       | [status] |
| Missing error handling   | [n]        | 15 (est.)   | 5 (est.)       | [status] |
| Security vulnerabilities | [n]        | 2 (est.)    | 0              | [status] |

---

## Next Steps

1. [✅ | ⏭️] Auto-fixes applied and verified
2. ⏭️ Manual review of remaining critical issues
3. ⏭️ Address antagonist new findings
4. ⏭️ Integration testing
5. ⏭️ Security audit (cargo-audit)

---

**End of Review Report**
````
