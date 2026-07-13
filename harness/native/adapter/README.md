# Native adapter

The Specify-owned native harness adapter is one deterministic, model-free library implementing both `specify:adapter` axes for engine tests. It supplies controlled survey/extract data (including a cross-source overlap, an authority disagreement, and an evidence gap), stable guidance, observable build output, and typed failures. The workflow crates' integration tests consume it through the test-only provider bridge at `crates/change/tests/common/fixture.rs`; the sibling [`../../wasm/adapter`](../../wasm/adapter/) package wraps the same core behind the WIT component boundary.

Behaviour keys off the routed adapter id (`source:<name>` / `target:<name>`), so one component artifact bound under several identities supplies every profile:

- a name containing `docs` or `code` selects the adversarial two-source pair: an overlapping `login-flow` lead in both, an authority disagreement on `session.timeout` (documentation says 30 minutes, behaviour says 15), and a deliberate evidence gap behind the docs-only `password-reset` lead;
- a name containing `fail-survey`, `fail-extract`, `fail-guidance`, `fail-build`, or `fail-merge` returns the matching typed failure;
- a name containing `missing-output` reports a successful build whose declared output was never written (the `target-build-output-missing` negative case);
- any other name selects the minimal single-lead `greeting` profile used by the deterministic full-loop tests.

A build additionally honours a per-project marker file (`FAIL_BUILD_MARKER`): when it exists at the project root the build returns a *failed report* (as opposed to a seam error), so interruption tests can park and resume a run without rebinding adapters. The phased merge gates honour the analogous `FAIL_MERGE_PREFLIGHT_MARKER` / `FAIL_MERGE_POSTFLIGHT_MARKER` pair.
