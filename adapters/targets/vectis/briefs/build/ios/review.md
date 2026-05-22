# Vectis build — iOS review

Loaded by [../../build.md](../../build.md) Step 11 after [iOS verify](write.md#verify-max-3-iterations) succeeds. Scope: every Swift file under `${IOS_SHELL_DIR}` plus read-only access to `${PROJECT_DIR}/shared/src/app.rs` and the wired UI input set (`composition.yaml`, `tokens.yaml`, `assets.yaml`).

Carries the body of the retired `vectis-ios-reviewer` skill. The iOS-specific team-spawn protocol lives in [`review/team-protocol-ios.md`](../../../../../../plugins/vectis/references/review/team-protocol-ios.md).

## Pipeline

1. **Verify prerequisites** — iOS verify succeeded; SwiftLint / swiftformat are available.
2. **Spawn specialists concurrently** with the verbatim prompts in [`review/team-protocol-ios.md`](../../../../../../plugins/vectis/references/review/team-protocol-ios.md):
   - **Structural** — IOS-001..019: ViewModel / screen correspondence, effect handlers, token usage, ScrollView hazards, recurring-group component candidates. Full library: [`review/ios-checks.md`](../../../../../../plugins/vectis/references/review/ios-checks.md).
   - **Quality** — SWF-001..010: concurrency, force unwraps, a11y labels, state management, previews, swiftformat. Full library: [`review/swift-quality-checks.md`](../../../../../../plugins/vectis/references/review/swift-quality-checks.md).
   - **Integration** — only on the first full-scope iteration. Token / asset / composition cross-artifact checks per [`review/team-protocol-ios.md`](../../../../../../plugins/vectis/references/review/team-protocol-ios.md) § Integration.
3. **Universal checks (lead).** Apply UNI-001..021 from the default codex with Swift heuristics. Full library: [`review/universal-checks.md`](../../../../../../plugins/vectis/references/review/universal-checks.md).
4. **Adversarial challenge.** Forward all findings to the antagonist per [`agent-teams.md`](../../../../../../plugins/vectis/references/agent-teams.md).
5. **Synthesis.** Lead authors the iteration report per [`review/iteration-report.md`](../../../../../../plugins/vectis/references/review/iteration-report.md).
6. **Mechanical auto-fixes (when safe).** Accessibility labels, design-token swaps, missing `#Preview`, Inject boilerplate. Revert the batch if `swiftformat` or the build regresses.

## Orchestrated mode

When the parent build brief passes `orchestrated: true`, the reviewer returns classified `design_findings` (`code-fix` vs `spec-change`) instead of writing a follow-up Specify slice. The parent consolidates findings across iOS and Android into one cross-platform finding set (see [../../build.md](../../build.md) § Consolidate review findings).

## Finding-ID conventions

- Report-local occurrence IDs: `IOS-1`, `SWF-1`, `INT-1`, `UNI-1`, `NEW-1`.
- Stable codex citations: `rule_id: VECTIS-IOS-001` (for example) appears alongside each mapped finding. Codex rules: [`adapters/targets/vectis/codex/`](../../../codex/).
- Severity reflects antagonist adjustments — upgrades and downgrades rewrite the displayed severity but preserve the original prefix and occurrence ID.
- Every finding carries a `file:line` reference and a verbatim code snippet.
