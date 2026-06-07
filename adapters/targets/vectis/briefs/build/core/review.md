# Vectis build — core review

Loaded by [../../build.md](../../build.md) Step 11 after the core verify-repair loop succeeds. Scope: the Rust `shared` crate. Drives an agent team — three specialists plus an antagonist — through a bounded review-fix loop (max 3 iterations).

The shared agent-team protocol lives in [`../../../references/agent-teams.md`](../../../references/agent-teams.md); the core-specific team-spawn protocol lives in [`review/team-protocol-core.md`](../../../references/review/team-protocol-core.md).

## Pipeline

1. **Verify prerequisites** — the core verify-repair loop returned `success`, `${PROJECT_DIR}/shared/` exists, and `cargo check` passes.
2. **Spawn specialists concurrently** with the verbatim prompts in [`review/team-protocol-core.md`](../../../references/review/team-protocol-core.md):
   - **Structural** — CRX-001..011: missing `render()`, serde derives, input validation, `PendingOp` timestamps, ViewModel typing, unused deps. Full library: [`review/crux-checks.md`](../../../references/review/crux-checks.md).
   - **Logic** — LOG-001..009: state-machine completeness, op coalescing, concurrent conflicts, temporal ordering, rapid-action sequences, spec gaps, spec-to-test coverage, stale tests. Full library: [`review/logic-checks.md`](../../../references/review/logic-checks.md).
   - **Quality** — GEN-001..012: no `unwrap` / `expect` outside test setup, no debug output, no hardcoded secrets, error propagation, match exhaustiveness, function length. Full library: [`review/general-checks.md`](../../../references/review/general-checks.md).
3. **Universal checks (lead).** Apply every `UNI-*` rule from [`adapters/shared/rules/universal/`](../../../../../shared/rules/universal/) with Rust / Crux heuristics. Full library: [`review/universal-checks.md`](../../../references/review/universal-checks.md). Skip universal checks already covered by the specialists per the dedupe table in [`review/team-protocol-core.md`](../../../references/review/team-protocol-core.md).
4. **Adversarial challenge.** Forward all findings to the antagonist. The antagonist confirms, upgrades, downgrades, disputes, and may add `NEW-` findings. Protocol: [`agent-teams.md`](../../../references/agent-teams.md).
5. **Synthesis.** Lead authors the iteration report per [`review/iteration-report.md`](../../../references/review/iteration-report.md).
6. **Mechanical auto-fixes (when safe).** Missing serde derives, `render().and(...)` wraps, `.trim()` / empty input checks, unused deps. Revert the full batch if `cargo check` / `cargo clippy` / `cargo test` regress.
7. **Logic findings stay non-mechanical.** Never auto-fix LOG-001..008 without explicit confirmation; surface them as design-level findings classified `code-fix` or `spec-change`.

## Standalone vs orchestrated

The core reviewer has no orchestrated mode — when design-level findings accumulate it always returns them for consolidation by the parent build brief / operator. Per-platform shell reviewers ([`../ios/review.md`](../ios/review.md), [`../android/review.md`](../android/review.md)) honour the `orchestrated: true` flag.

## Finding-ID conventions

- Report-local occurrence IDs: `CRX-1`, `LOG-1`, `GEN-1`, `UNI-1`, `NEW-1`. These are **report-local** counters — the `id` field on a structured `LintFinding` (the `LintFinding` schema uses the equivalent `FIND-0001` shape; this review uses prefixed counters for human triage). They restart in each report and must not be confused with codex `rule-id`s.
- Stable codex citations: `rule_id: VECTIS-001` (for example) appears alongside each mapped finding. Codex ids must match `^VECTIS-[0-9]{3}$`. Markdown `rule_id:` prose maps to the kebab-case `rule-id` field on the `LintFinding` wire shape (likewise `target-adapter`, `source-adapter`, `related-rule-ids`, `confidence`). Rules: [`adapters/targets/vectis/rules/`](../../../rules/).

Severity values in finding output use the closed `LintFinding` severity enum: `critical`, `important`, `suggestion`, `optional`.

See [iteration-report.md](../../../references/review/iteration-report.md) § Finding-ID conventions for severity and `file:line` rules.
