# Vectis build — iOS review

Loaded by [../../build.md](../../build.md) Step 11 after [iOS verify](write.md#verify-max-3-iterations) succeeds. Scope: every Swift file under `${IOS_SHELL_DIR}` plus read-only access to `${PROJECT_DIR}/shared/src/app.rs` and the wired UI input set (`composition.yaml`, `tokens.yaml`, `assets.yaml`).

The iOS-specific team-spawn protocol lives in [`review/team-protocol-ios.md`](../../../references/review/team-protocol-ios.md).

## Pipeline

1. **Verify prerequisites** — iOS verify succeeded; SwiftLint / swiftformat are available.
2. **Spawn specialists concurrently** with the verbatim prompts in [`review/team-protocol-ios.md`](../../../references/review/team-protocol-ios.md):
   - **Structural** — IOS-001..019: ViewModel / screen correspondence, effect handlers, token usage, ScrollView hazards, recurring-group component candidates. Full library: [`review/ios-checks.md`](../../../references/review/ios-checks.md).
   - **Quality** — SWF-001..010: concurrency, force unwraps, a11y labels, state management, previews, swiftformat. Full library: [`review/swift-quality-checks.md`](../../../references/review/swift-quality-checks.md).
   - **Integration** — only on the first full-scope iteration. Token / asset / composition cross-artifact checks per [`review/team-protocol-ios.md`](../../../references/review/team-protocol-ios.md) § Integration, including **IOS-020** (render-by-`kind`, codex `VECTIS-006`) in [`review/ios-checks.md`](../../../references/review/ios-checks.md).
3. **Universal checks (lead).** Apply every `UNI-*` rule from [`adapters/shared/rules/universal/`](../../../../../shared/rules/universal/) with Swift heuristics. Full library: [`review/universal-checks.md`](../../../references/review/universal-checks.md).
4. **Adversarial challenge.** Forward all findings to the antagonist per [`agent-teams.md`](../../../references/agent-teams.md).
5. **Synthesis.** Lead authors the iteration report per [`review/iteration-report.md`](../../../references/review/iteration-report.md).
   - Return classified `design_findings` per [../../build.md](../../build.md) § Consolidate review findings.
6. **Mechanical auto-fixes (when safe).** Accessibility labels, design-token swaps, missing `#Preview`, Inject boilerplate. Revert the batch if `swiftformat` or the build regresses.

## Finding-ID conventions

- Report-local occurrence IDs: `IOS-1`, `SWF-1`, `INT-1`, `UNI-1`, `NEW-1`. These are **report-local** counters — the `id` field on a structured `LintFinding` (the `LintFinding` schema uses the equivalent `FIND-0001` shape; this review uses prefixed counters for human triage). They restart in each report and must not be confused with codex `rule-id`s.
- Stable codex citations: `rule_id: VECTIS-101` (for example) appears alongside each mapped finding. Codex ids must match `^VECTIS-[0-9]{3}$`. Markdown `rule_id:` prose maps to the kebab-case `rule-id` field on the `LintFinding` wire shape (likewise `target-adapter`, `source-adapter`, `related-rule-ids`, `confidence`). Rules: [`adapters/targets/vectis/rules/`](../../../rules/).

Severity values in finding output use the closed `LintFinding` severity enum: `critical`, `important`, `suggestion`, `optional`.

See [iteration-report.md](../../../references/review/iteration-report.md) § Finding-ID conventions for severity and `file:line` rules.
