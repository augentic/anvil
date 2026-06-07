# Omnia build — standards review (code reviewer)

Loaded by [../build.md](../build.md) phase 6, after the verify-repair loop succeeds. Applies **engineering standards** with model-assisted judgment: an agent team of three specialists (Security, Correctness, Quality) plus an antagonist; the lead synthesises findings into `$REVIEW_OUTPUT = $CRATE_PATH/REVIEW.md`. This is build-time standards application, not CI-native `specify lint` and not plan Gate 1.

## Review pipeline

1. **Verify prerequisites** — `cargo check` passes (the verify-repair loop already guarantees this) and `$CRATE_PATH` exists. Resolve the optional `fix` flag.
2. **Spawn specialists concurrently** using the verbatim prompts in [`team-protocol-crate.md`](../../references/team-protocol-crate.md):
   - Security Reviewer — SEC-prefixed findings.
   - Correctness Reviewer — COR-prefixed findings.
   - Quality Reviewer — QUA-prefixed findings.
   The full check library per specialist (SEC / COR / QUA categories) lives in [`review-categories.md`](../../references/review-categories.md).
3. **Universal checks (lead)** — apply every `UNI-*` rule from [`adapters/shared/rules/universal/`](../../../../shared/rules/universal/) with Omnia / WASM heuristics; prefix `UNI-`. Skip universal checks already covered by SEC / COR / QUA per the table in [`review-categories.md`](../../references/review-categories.md).
4. **Adversarial challenge** — forward all findings to the antagonist. The antagonist confirms, upgrades, downgrades, disputes, and may add `NEW-` findings. Protocol: [`team-protocol-crate.md`](../../references/team-protocol-crate.md).
5. **Synthesis** — author `REVIEW.md` per the template in [`review-output-template.md`](../../references/review-output-template.md). Sections: Summary, Findings (grouped by severity), Adversarial Review (confirmed / downgraded / upgraded / disputed / new tallies), Auto-Fix Summary (when `fix` is set), Quality Metrics.
6. **Auto-fix (only when `fix`)** — apply safe fixes for confirmed / upgraded auto-fixable findings only. Scope, success-rate table, and revert-on-failure recipe: [`review-auto-fix.md`](../../references/review-auto-fix.md). Re-run `cargo check`; revert on failure. Respect antagonist regression flags.

## Finding-ID conventions

- Report-local occurrence IDs: `SEC-1`, `COR-1`, `QUA-1`, `UNI-1`, `NEW-1`. These are the `id` field on a structured `LintFinding` (the `LintFinding` schema uses the equivalent `FIND-0001` shape; this report uses prefixed counters for human triage).
- Stable codex citations: `rule_id: OMNIA-002` (for example) appears alongside each mapped finding. The markdown `rule_id:` prose maps to the kebab-case `rule-id` field on the `LintFinding` wire shape. Omnia-specific rules live under [`adapters/targets/omnia/rules/`](../../rules/): `OMNIA-001` Provider-Only Host Access, `OMNIA-002` WASM Guest Runtime Constraints, `RUST-001` Classified SDK Errors / No Panic Paths, `SEC-001` Host-Managed Secrets and Identity. All codex ids are three digits and match `^(UNI|SRC|FRAME|RUST|IFACE|SEC|OMNIA|VECTIS|ORG)-[0-9]{3}$`.
- Severity uses the closed `LintFinding` severity enum: `critical`, `important`, `suggestion`, `optional`. Antagonist adjustments rewrite the displayed severity but preserve the original prefix and occurrence ID.
- Every finding carries a `file:line` reference and a verbatim code snippet.

## Auto-fix scope

Auto-fix applies only to findings the antagonist confirmed or upgraded, and only to the auto-fixable categories listed in the per-category success-rate table in [`review-auto-fix.md`](../../references/review-auto-fix.md). Auto-fix runs after synthesis, before the report is finalised. If `cargo check` fails after a fix is applied, the fix reverts and the finding is left for manual handling.

- **`critical` / `important`** findings not auto-fixed are left for the operator.
- **`suggestion`** findings without an auto-fix are documented as accepted technical debt with rationale.
- **`optional`** findings are reported and require no action.

## Remediation cycle

After auto-fix completes:

1. Parse `$REVIEW_OUTPUT`. Process by severity.
2. **`critical` / `important`** — auto-fixable + not disputed: apply the fix directly. Non-auto-fixable: classify as test issue vs code issue and re-enter the matching phase brief (back to [crate writer](crate.md) or [test writer](test.md)). After all `critical` / `important` fixes, return to the parent brief's verify-repair loop with max 2 iterations (tighter than the standard 3, since these are targeted repairs).
3. **`suggestion`** — auto-fix when available; otherwise document as accepted technical debt in `REVIEW.md`.
4. **`optional`** — document only.
5. Re-run this brief (without `fix`) to verify fix quality. If new `critical` / `important` findings appear, repeat the remediation cycle once.

## See also

- [`review-categories.md`](../../references/review-categories.md) — full SEC/COR/QUA/UNI check library, Omnia/WASM heuristics, codex `rule_id` mapping guidance.
- [`team-protocol-crate.md`](../../references/team-protocol-crate.md) — verbatim specialist spawn prompts, antagonist protocol, synthesis rules.
- [`review-auto-fix.md`](../../references/review-auto-fix.md) — `fix` scope, success-rate table, regression guard, recovery process.
- [`review-output-template.md`](../../references/review-output-template.md) — `REVIEW.md` template and finding-ID conventions.
- [`agent-teams.md`](../../references/agent-teams.md) — shared team roles, antagonist protocol, file ownership.
- [`codex/`](../../rules/) — Omnia-specific rules cited as `rule_id` values.
- [`codex/universal/`](../../../../shared/rules/universal/) — shared `UNI-*` rules.
