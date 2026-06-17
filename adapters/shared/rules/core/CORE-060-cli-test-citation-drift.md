---
id: CORE-060
title: CLI Test-Citation Drift
severity: important
trigger: Documentation claims a behavior is proven by a named CLI test that the pinned binary's test tree does not contain.
rule_hints:
  - kind: path-pattern
    value: "docs/**/*.md"
  - kind: path-pattern
    value: "AGENTS.md"
  - kind: cli-contract
    value: test-citations
    description: "`tests/….rs` inline code spans and CLI-repo link targets under `tests/` must exist in the binary's build-time test inventory."
    config:
      link-prefixes:
        - "https://github.com/augentic/specify/blob/main/engine/"
        - "https://github.com/augentic/specify/tree/main/engine/"
---

## Rule

This repository routinely punts proof to the CLI: "the deterministic substrate is proven by named tests in the `engine/` workspace", followed by a citation like `tests/plan/end_to_end.rs` or a GitHub link into the `engine/tests/` tree. Those claims rot silently when a test file is renamed, split, or deleted — the prose keeps asserting coverage that no longer exists under that name.

The `test-citations` selector of the `cli-contract` kind closes the gap. The pinned binary embeds an inventory of its own `tests/` tree at build time (published as the `tests` array of `specify contract dump`), and every citation in scope is checked against it:

- An inline code span matching `tests/….rs` must be a file in the inventory.
- A link target under one of the configured CLI-repo prefixes pointing into `tests/` must resolve — a file citation exactly, a directory citation by containing at least one inventoried file. `#L…` fragments are ignored.

The scope is deliberately narrower than [CORE-057](CORE-057-cli-contract-drift.md): only `docs/**` and `AGENTS.md`, because adapter references and briefs legitimately describe `tests/` layouts of *generated downstream crates* (`tests/provider.rs` in an Omnia consumer project) that are not CLI tests and must not be checked against the CLI inventory.

## Look For

- "Proven by `tests/workflow/propose.rs`" where the module moved to a different file.
- A `blob/main/engine/tests/…` GitHub link whose target was deleted in the pinned CLI source.
- A `tree/main/engine/tests/fixtures/…` directory link whose fixtures were re-homed.

## Fix

Re-point the citation at the test that now carries the behavior — `specify contract dump --format json | jq '.tests'` lists the pinned binary's full inventory. If the named test was retired without replacement, rewrite the prose so it no longer claims coverage. If a citation legitimately refers to a non-CLI test tree from within scope, add the exact path to the rule's `config: ignore`.
