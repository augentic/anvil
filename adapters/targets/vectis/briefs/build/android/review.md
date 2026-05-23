# Vectis build — Android review

Loaded by [../../build.md](../../build.md) Step 11 after [Android verify](write.md#verify-max-3-iterations) succeeds. Scope: every Kotlin file under `${ANDROID_SHELL_DIR}` plus read-only access to `${PROJECT_DIR}/shared/src/app.rs` and the wired UI input set (`composition.yaml`, `tokens.yaml`, `assets.yaml`).

Carries the body of the retired `vectis-android-reviewer` skill. The Android-specific team-spawn protocol lives in [`review/team-protocol-android.md`](../../../references/review/team-protocol-android.md).

## Pipeline

1. **Verify prerequisites** — Android verify succeeded; ktlint / detekt are available.
2. **Spawn specialists concurrently** with the verbatim prompts in [`review/team-protocol-android.md`](../../../references/review/team-protocol-android.md):
   - **Structural** — AND-001..027: screen / ViewModel correspondence, effect handlers, token usage, UniFFI library override, generated-type imports, coroutine safety, recurring-group component candidates. Full library: [`review/android-checks.md`](../../../references/review/android-checks.md).
   - **Quality** — KTL-001..010: force-unwraps, debug output, coroutine cancellation, Compose state, previews, a11y `contentDescription`. Full library: [`review/kotlin-quality-checks.md`](../../../references/review/kotlin-quality-checks.md).
   - **Integration** — only on the first full-scope iteration. Token / asset / composition cross-artifact checks per [`review/team-protocol-android.md`](../../../references/review/team-protocol-android.md) § Integration.
3. **Universal checks (lead).** Apply every `UNI-*` rule from [`adapters/shared/codex/universal/`](../../../../../shared/codex/universal/) with Kotlin / Android heuristics. Full library: [`review/universal-checks.md`](../../../references/review/universal-checks.md).
4. **Adversarial challenge.** Forward all findings to the antagonist per [`agent-teams.md`](../../../references/agent-teams.md).
5. **Synthesis.** Lead authors the iteration report per [`review/iteration-report.md`](../../../references/review/iteration-report.md).
6. **Mechanical auto-fixes (when safe).** `contentDescription`, design-token swaps, missing `@Preview`, generated-FFI-type imports (`import com.vectis.<app>.*`), `CancellationException` rethrow, replacing stale `import com.vectis.design.*` with `import com.vectis.<app>.ui.theme.*`. Revert the batch if the Gradle build regresses.

## Orchestrated mode

The reviewer always returns classified `design_findings` (`code-fix` vs `spec-change`) for the parent build brief to consolidate across iOS and Android into one cross-platform finding set (see [../../build.md](../../build.md) § Consolidate review findings). The legacy "reviewer auto-creates a Specify change" path is retired in 2.0 — follow-up work is queued as a new slice via the operator's normal `/spec:plan` flow.

## Finding-ID conventions

- Report-local occurrence IDs: `AND-1`, `KTL-1`, `INT-1`, `UNI-1`, `NEW-1`.
- Stable codex citations: `rule_id: VECTIS-AND-001` (for example) appears alongside each mapped finding. Codex rules: [`adapters/targets/vectis/codex/`](../../../codex/).
- Severity reflects antagonist adjustments — upgrades and downgrades rewrite the displayed severity but preserve the original prefix and occurrence ID.
- Every finding carries a `file:line` reference and a verbatim code snippet.
