# RM-01 Harness Implementation Review

> Review notes for the current `tests/` implementation against
> [`rm-01-harness.md`](rm-01-harness.md). Use this as a worklist for hardening
> the acceptance layer.

## Summary

The current `tests/cross_repo.ts` suite is valuable and should be kept. It
creates a realistic local hub, two fixture repos, fake GitHub remotes, fake
`gh`, and drives the real `specify` binary through registry setup, workspace
sync, a contract-first routed plan, branch preparation, push, external merge
simulation, and finalize.

However, it is closer to a repository-level CLI substrate replay than to the
skill/capability integration harness described in `rm-01-harness.md`. A green
run proves that the CLI surfaces compose correctly, but it does not yet prove
that workflow skills, capability briefs, or specialist generators can drive
that substrate.

## Findings

### High: The Harness Does Not Exercise The RFC's Main Skill Path

`rm-01-harness.md` defines the missing layer as:

```text
change brief/docs
  -> /change:plan
  -> capability brief pipelines
  -> /change:execute loop
  -> /spec:define
  -> /spec:build
  -> specialist skills
  -> /spec:merge
  -> workspace push
  -> external PR merge simulation
  -> change finalize
```

The current implementation manually creates the plan with `specify change plan
add`, then writes deterministic slice artifacts directly in TypeScript. That is
useful for stabilizing CLI behavior, but it will not catch drift in:

- `/change:plan` planning from briefs/docs
- `/change:execute loop`
- `/spec:define`, `/spec:build`, and `/spec:merge`
- capability brief pipelines
- contract authoring/verification
- Omnia and Vectis specialist output

Recommended next step: preserve `tests/cross_repo.ts` as the cheap substrate
acceptance test, then add a second staged harness that starts with real
`/change:plan` or a recorded/fixture runner for slash-command behavior.

### High: Acceptance Can Silently Skip

`tests/cross_repo.ts` returns successfully when no suitable `specify` binary is
found or when the binary lacks required surface. This is convenient locally but
risky for CI and for explicit `SPECIFY_BIN` runs.

Recommended change:

- keep local skip behavior when `SPECIFY_BIN` is unset and `specify` is absent
- fail when `SPECIFY_BIN` is set but invalid
- fail in CI if the required surface is missing
- avoid falling back to `PATH` after an explicit invalid `SPECIFY_BIN`

### Medium: Some CLI JSON Contract Assertions Were Lost

The Rust substrate test in `specify-cli/tests/cross_repo.rs` asserts a few JSON
surface details that the Deno test does not currently check directly:

- `change plan next` exposes useful fields such as sources and description
- `workspace prepare-branch --format json` returns the expected `prepared`,
  `branch`, and `project`
- `workspace status --format json` reports clean clone state, correct branch,
  project config presence, and branch/change alignment
- `change finalize --format json` reports merged project statuses, summary
  counts, and the archived path

Recommended change: add these assertions to `tests/cross_repo.ts`. They are
low-cost and protect surfaces workflow skills consume.

### Medium: Evidence Capture Is Thinner Than The RFC

The support layer captures stdout/stderr logs, but `rm-01-harness.md` asks for
debugging artifacts such as a transcript, tool calls, final tree, plan copy,
registry, workspace status, push output, finalize output, and failure evidence.

Recommended first increment:

- write `final-tree.txt` on failure
- save `workspace-status.json`
- save `push-output.json`
- save `finalize-output.json`
- keep stdout/stderr logs

This would make CI artifacts more useful without requiring a full transcript
runner yet.

### Medium: CI Uses `specify-cli` `main` Despite Saying It Is Pinned

`.github/workflows/acceptance.yml` says the CLI ref is pinned, but
`SPECIFY_CLI_REF` is currently `main`. That makes failures sensitive to upstream
movement.

Recommended change: either pin to a commit/tag or update the comment and docs to
make the floating compatibility contract explicit.

### Low: Roadmap And RFC Wording Need Reconciliation

The roadmap and RFC should distinguish three layers clearly:

- Rust CLI substrate test in `specify-cli`
- current Deno repo-level cross-repo acceptance in `specify/tests`
- future skill/capability replay harness

Recommended change: update roadmap/RFC text so the current Deno suite is not
mistaken for the full skill/capability harness.

## TypeScript Versus Rust

The current Deno acceptance test could be rewritten in Rust because it mainly
uses subprocesses, Git, temp directories, fake `gh`, and filesystem assertions.
There is no technical blocker.

It is not obviously worth doing now:

- this repo already uses Deno for `scripts/checks.ts` and `make test`
- repo-level workflow acceptance does not need to share a language with the Rust
  CLI implementation
- future skill/agent replay is likely easier from TypeScript, especially if it
  uses a Cursor SDK or transcript runner
- low-level CLI behavior already has a natural Rust home in `specify-cli`

Recommended posture: keep CLI substrate tests in Rust in `specify-cli`; keep
repo-level workflow and future skill/agent acceptance in Deno/TypeScript unless
the harness becomes purely CLI-only.

## `capabilities/contracts/tests/`

These files are still required. They are not duplicates of
`tests/cross_repo.ts`.

`capabilities/contracts/tests/` is the owner-local scenario pack for
`contracts@v1`. It documents manual or agent-run scenarios for the contract
slice loop:

- `/spec:define`
- `/spec:build`
- `/spec:merge`
- expected `contracts/**` artifacts
- contract verifier expectations
- negative boundary behavior

`tests/cross_repo.ts` only stubs a tiny contract artifact and does not exercise
contract capability generation, import, extraction, repair, or validation. The
contracts scenarios remain the only documented acceptance oracle for that
capability.

Recommended change: keep the directory, but make its status explicit wherever
needed:

- scenario frontmatter is statically validated by `make checks`
- scenario bodies are currently manual instructions
- future automation can promote selected scenarios to `backend: agent`,
  `backend: recorded`, or `backend: fixture`

## Suggested Worklist

1. Harden skip behavior for `SPECIFY_BIN` and CI.
2. Restore missing JSON surface assertions from the Rust substrate test.
3. Save minimal structured failure evidence from the Deno harness.
4. Clarify roadmap/RFC/docs language around substrate replay versus
   skill/capability replay.
5. Add a Phase 1 skill-runner spike for real `/change:plan` from the OAuth
   brief, or document why a recorded/test-double runner is the next step.
6. Keep `capabilities/contracts/tests/` and consider automating one happy-path
   contracts scenario before attempting full Omnia/Vectis generation.
