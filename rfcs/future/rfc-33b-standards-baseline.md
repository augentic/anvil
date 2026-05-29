# RFC-33b: Standards Baseline

> Status: Deferred · Depends: [RFC-28](../done/rfc-28-standards-contract.md), [RFC-32](../done/rfc-32-standards-enforcement.md), [RFC-33a](../rfc-33a-ignore-directives.md) · Enables: [roadmap RM-10](../roadmap.md#rm-10-ci-native-standards-enforcement) (when triggered), [roadmap RM-14](../roadmap.md#rm-14-local-structured-workflow-events)

## Abstract

[RFC-33a](../rfc-33a-ignore-directives.md) delivers in-source ignore directives, the initial `status` enum widening (adding `ignored`), the `disposition` field with the `directive?` sub-field, and the `lint-completed` journal event. It does not deliver the **cross-run** lifecycle: how a project acknowledges a body of legitimate findings as baseline debt, how scans diff against prior runs, and how operators stage remediation across releases.

This RFC pre-designs that surface so it can land against a settled contract when one of the trigger conditions in §"Trigger conditions" is met. While deferred, no Phase 2 code, schema file, CLI verb, or filesystem path under `.specify/lint/` ships. RFC-33a established the partial wire shape (D5 / D6 — adding `ignored` to `status` and `directive?` to `disposition`); RFC-33b extends both decisions additively when it lands, adding `new` / `baselined` to `status` and `baseline?` to `disposition`.

Concretely, RFC-33b adds:

1. **Baseline file** — `.specify/lint/baseline.json`, indexed by RFC-28 `fingerprint`. `specrun lint run` runs in baseline mode by default when the file is present; only findings outside the baseline block CI.
2. **Last-run persistence** — `.specify/lint/last.json` carries the full envelope from the most recent run so `specrun lint baseline diff` answers "what changed since last scan" without re-scanning.
3. **Diff verb** — `specrun lint baseline diff` reports `new[] / fixed[] / unchanged[] / ignored[] / baselined[]` as a pure function over `last.json`, the current scan, and the baseline.
4. **CLI surface** — `specrun lint baseline {write, drop, diff}` subcommands plus `--no-baseline` / `--baseline <path>` flags on `specrun lint run`.
5. **Status enum widening** — widens `schemas/diagnostics/diagnostic.schema.json` a second time, adding `new` and `baselined` to the `status` enum (extending RFC-33a's initial widening) and the optional `disposition.baseline` sub-field. Also supplies producers for the pre-existing `fixed` and `accepted` values inherited from RFC-28.

This RFC adds no lifecycle authority. Baselined findings never transition plan entries, slices, or changes. Baseline mode changes which findings count as CI blockers; it does not change the meaning of a finding.

## Motivation

[RFC-33a](../rfc-33a-ignore-directives.md) §"Motivation" enumerates three pressure points after RFC-32 lands. The first (intentional exceptions need to live with the code) is addressed by RFC-33a's ignore directives. This RFC addresses the remaining two:

- **Mass adoption requires a staging mechanism.** A consumer project picking up RFC-32 on a codebase older than the rule set will see findings for code that was correct at the time it was written. Without baselines, the operator's only choices are to fix everything before turning on CI gates or to disable entire rules globally. The first is impractical; the second discards the rule's value forever.
- **CI dashboards need diffs, not absolute counts.** "Project X has 412 findings" is not actionable. "PR Y introduced 3 new findings and fixed 1" is. Stable fingerprints are already there per RFC-28; what is missing is somewhere to compare them against.

Both pressure points are conditional: neither bites under a "resolve every finding before release" operator policy on a Specify-native codebase. The trigger conditions below name what changes that.

### Trigger conditions

Any one of the following promotes RFC-33b from deferred to active:

1. **Retroactive rule pressure.** A new `UNI-*`, `OMNIA-*`, `VECTIS-*`, or other shared rule lands that flags existing slice code with no immediate-fix path. The operator needs a way to acknowledge the debt while the cleanup is tracked in plan entries.
2. **Pre-existing codebase intake.** A consumer project onboards code that did not flow through `/spec:build` — for example the TypeScript→Rust WASM reconstruction input named in `.cursor/rules/project.mdc` §"Main Inputs". Day-one scans on that input are expected to surface mass legitimate findings that cannot be cleared before CI is enabled.
3. **Operator policy change.** "Resolve every finding before release" stops being feasible at the team's current cadence, and staged remediation across releases becomes the norm. At that point the `last.json` + diff verb signal becomes non-trivial.

Each trigger has a clean owner: (1) is detected by codex resolver output deltas, (2) is detected by the source adapter onboarding the codebase (`code-typescript`, future `code-java`, …), (3) is an explicit operator decision documented alongside the project's release runbook.

### What this RFC does not repeat

- **RFC-28** owns the `LintFinding` envelope, the fingerprint algorithm, the severity enum, and the closed `rule-id` namespace. RFC-33b consumes the fingerprint as its join key; it makes no envelope changes.
- **RFC-32** owns `WorkspaceModel`, the hint interpreter, and `specrun lint`'s single-scan behaviour. RFC-33b adds one optional baseline-loading step to the scanner pipeline, behind the same `scan_profile: consumer` flag.
- **RFC-33a** owns the ignore directive grammar, the initial `status` enum widening (adding `ignored`), the `disposition` field with `directive?` sub-field (D5, D6), and the `lint-completed` journal event (D8). RFC-33b widens `finding.schema.json` again to add `new` and `baselined` to the enum and `baseline?` to `disposition`, and supplies producers for the pre-existing `fixed` and `accepted` values.
- **RFC-14 / RM-14** owns workflow telemetry. RFC-33b flips `baseline_present` in the existing `lint-completed` event payload from always-`false` to scan-derived; it does not define a telemetry sink, query tool, or aggregation surface.

## Principles

The principles that govern this layer are stated in full in [RFC-33a](../rfc-33a-ignore-directives.md) §"Principles". RFC-33b activates the two that were dormant while it was deferred:

- **(2) Baseline is opt-in but on-by-default once present.** The first scan after `specrun lint baseline write` runs in baseline mode without a flag; explicit `--no-baseline` is required to ignore.
- **(6) One writer per file.** `.specify/lint/baseline.json` is only written by `specrun lint baseline write`. `.specify/lint/last.json` is only written by `specrun lint run`. No skill, no agent, no other CLI verb edits either file.

Principles (1), (3), (4), (5), and (7) carry forward unchanged.

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Baseline file** | A scan-profile-scoped baseline lives at `.specify/lint/baseline.json`. | New `schemas/lint/baseline.schema.json`; new `Baseline` DTO in `specify-lints`; `specrun lint run` loads and matches by `fingerprint`. |
| **D2 Baseline mode default** | When a baseline file matching the active scope exists under `.specify/lint/`, `specrun lint run` runs in baseline mode unless `--no-baseline` is passed. Per-target files (`baseline.<target>.json`) override the project-wide `baseline.json` only when `--target <name>` is active; nearest-scope wins. `--baseline <path>` is an explicit override that bypasses the layering rule. | Two new flags on the `specrun lint run` clap surface (`--no-baseline`, `--baseline <path>`); one new branch in the scanner pipeline; selection table pinned in §"Baseline file" below. |
| **D7 Last-run persistence** | `.specify/lint/last.json` holds the previous run's envelope verbatim. | One additional write at scanner exit; one new schema `schemas/lint/run.schema.json` (same shape as live emission). |
| **D9 Diff verb** | `specrun lint baseline diff` reports `new[]`, `fixed[]`, `unchanged[]`, `ignored[]`, `baselined[]` against `last.json`. | One new subcommand; pure function over two envelopes plus the baseline; no scan side effects. |
| **D10 Baseline omits rule body** | Baseline entries store only `(fingerprint, rule_id, path, line, kind, rationale, recorded_at)`. They never embed hint kind, regex pattern, or any other rule-body slice; interpreter drift surfaces through `evidence-payload` changes already encoded in the fingerprint. | `schemas/lint/baseline.schema.json` pins the closed entry shape; no duplicate of `specrun rules export` output; baseline rewrites are fingerprint-driven, not rule-edit-driven. |
| **D11 `baseline write` reads `last.json`** | `specrun lint baseline write` reads the most recent run from `.specify/lint/last.json` and refuses if the file is missing or stale (mtime older than the most recent file mtime under any scoped artifact path). It does not re-scan unless `--rescan` is passed, in which case it runs a scan inline and writes the resulting envelope verbatim before persisting the baseline. | Closed input contract: scanner is the only writer of `last.json`; baseline write is a pure transformation of an already-approved envelope; `--rescan` flag covers the rare "fresh capture" path without forking the contract. |

The decision IDs (D1, D2, D7, D9, D10, D11) are preserved from the original combined RFC-33 so prior references continue to resolve. [RFC-33a](../rfc-33a-ignore-directives.md) owns D3, D4, D5, D6, D8, D12, D13.

### Baseline file

```json
{
  "version": 1,
  "created_at": "2026-05-27T20:42:00Z",
  "scan_profile": "consumer",
  "scope": {
    "target": "omnia",
    "artifact": null
  },
  "entries": [
    {
      "fingerprint": "sha256:…",
      "rule_id": "UNI-014",
      "path": "crates/billing/src/config.rs",
      "line": 18,
      "recorded_at": "2026-05-27T20:42:00Z",
      "kind": "accepted",
      "rationale": "Legacy endpoint; tracked in plan slice billing-config-extraction."
    }
  ]
}
```

Required behaviour:

- Entries are sorted by `(rule_id, path, line, fingerprint)` for byte-stable diffs.
- `kind` is one of `accepted | false-positive`. `accepted` is the default for `specrun lint baseline write`; `false-positive` requires `--false-positive` and a non-empty `--rationale`.
- `path`, `line`, and `rule_id` are denormalised from the matched finding for human review; the join key is `fingerprint` only.
- A baseline matches a finding when fingerprints are byte-equal. `path` / `line` drift is invisible because RFC-28's fingerprint excludes producer-local fields and includes `evidence-payload`, which captures the underlying code change.

Multiple baselines are supported by suffixing scope: `.specify/lint/baseline.<target>.json` (e.g. `baseline.omnia.json`). The unsuffixed file is the project-wide baseline; suffixed files override per-target during target-scoped scans. Layering rule: nearest-scope wins.

Selection matrix (per D2):

| Active scope | Files present | File matched against scan |
| --- | --- | --- |
| `specrun lint run` (no `--target`) | `baseline.json` only | `baseline.json` |
| `specrun lint run` (no `--target`) | `baseline.json` + `baseline.<target>.json` | `baseline.json` only — target-suffixed files are ignored when no target is active |
| `specrun lint run --target omnia` | `baseline.json` only | `baseline.json` |
| `specrun lint run --target omnia` | `baseline.json` + `baseline.omnia.json` | `baseline.omnia.json` only — per-target supersedes project-wide |

Override flags:

- `--baseline <path>` selects a specific file regardless of suffix; it bypasses the matrix entirely and is intended for CI dry-runs and ad-hoc comparisons.
- `--no-baseline` defeats every baseline file regardless of suffix; the run reports all findings as `open`.

### Last-run persistence

`.specify/lint/last.json` is the verbatim envelope from the most recent `specrun lint run` invocation (any flags, any scope). One file per project. It is overwritten on every run.

Diff semantics:

- `new[]` — fingerprints in current scan, not in `last.json`.
- `fixed[]` — fingerprints in `last.json`, not in current scan.
- `unchanged[]` — fingerprints in both.
- `ignored[]`, `baselined[]` — fingerprints in current scan with the matching status.

The diff verb is a pure function over `last.json`, the current scan, and the baseline. It never re-scans and never writes either file.

### CLI surface

```bash
specrun lint run --no-baseline                    # ignore every baseline file regardless of suffix
specrun lint run --baseline path/to/file.json     # explicit override; bypasses the §"Baseline file" matrix
specrun lint baseline write                   # consume .specify/lint/last.json and write the baseline
specrun lint baseline write --rescan          # scan inline, persist last.json, then write the baseline
specrun lint baseline write --rationale "Legacy import; tracked in slice X."
specrun lint baseline write --false-positive --rationale "Match in vendored fixture."
specrun lint baseline diff                    # compare current scan to last.json + baseline
specrun lint baseline diff <fingerprint>      # focus the diff on one finding
specrun lint baseline drop                    # remove the baseline file (journals the action)
```

Behavioural notes:

- `specrun lint baseline write` reads `.specify/lint/last.json` rather than re-scanning (D11). It refuses with `lint-baseline-write-stale` when `last.json` is missing, or when its mtime is older than the most recent file mtime under any scoped artifact path. Operators recover by re-running `specrun lint run` or by passing `--rescan` to capture and persist a fresh envelope inline.
- `specrun lint baseline write` requires `--yes` in non-TTY environments to confirm the silent acceptance of every current finding. In TTY mode it prints a count summary and asks for confirmation.
- `specrun lint baseline write --append <fingerprint>` adds one entry to an existing baseline without reading `last.json`; useful when an operator wants to accept a known finding by fingerprint alone.
- `specrun lint baseline drop` emits a `lint-completed` event with `baseline_present: false` and a synthetic count of zero, so the journal records the policy change.

### Schema changes

Two new schema files:

| File | Status | Owner |
| --- | --- | --- |
| `schemas/lint/baseline.schema.json` | New | RFC-33b |
| `schemas/lint/run.schema.json` | New | RFC-33b (matches the live envelope shape) |

RFC-33b widens `schemas/diagnostics/diagnostic.schema.json` a second time: the `status` enum adds `new` and `baselined`, and the `disposition` object gains the optional `baseline?` sub-field. The widening is additive and the fingerprint algorithm is unchanged.

### Implementation plan

1. **Schemas.** Add `schemas/lint/baseline.schema.json` and `schemas/lint/run.schema.json`. Widen `schemas/diagnostics/diagnostic.schema.json` additively: extend the `status` enum with `new` and `baselined`, and add the optional `disposition.baseline` sub-field.
2. **Standards-layer types.** Add `Baseline`, `BaselineEntry`, `ReviewRun` DTOs to `specify-lints`. Reuse RFC-28's canonical-JSON helper.
3. **Scanner pipeline.** Insert the baseline pass after directive matching. Order becomes: hint evaluation → default `status: open` assignment → directive validation/matching → baseline matching → ordering → envelope/render → status-aware exit decision.
4. **Last-run persistence.** Write `.specify/lint/last.json` at scanner exit, after envelope emission.
5. **CLI surface.** Add `specrun lint baseline {write, write --append, write --rescan, drop, diff}` subcommands and the `--no-baseline` / `--baseline <path>` flags on `specrun lint run`. Enforce the §"Baseline file" selection matrix, the `last.json` staleness check from D11 (`lint-baseline-write-stale`), and confirm-or-`--yes` discipline on every write.
6. **Journal.** Flip the `baseline_present` field to populate from the scan's actual baseline state. Add `counts.{new, baselined}` to the `lint-completed` payload, populated from the baseline pass output.
7. **Acceptance.** Golden tests: scan with baseline absent → all `open` (RFC-33a behaviour preserved); scan with baseline present → mix of `baselined` and `new`; diff across two scans → stable `new[]` / `fixed[]` / `unchanged[]`; per-target layering per the §"Baseline file" matrix.

### Migration

**For operator projects:** Additive. Without a baseline file, behaviour is identical to [RFC-33a](../rfc-33a-ignore-directives.md). To adopt baseline mode, run `specrun lint run` (which writes `.specify/lint/last.json`) then `specrun lint baseline write`, review the file in PR, and commit.

**For CLI maintainers:** The baseline and run files live under `.specify/lint/` — a new sub-tree, distinct from `.specify/cache/`, `.specify/slices/`, and `.specify/archive/`. Document the directory in `docs/reference/directory-layout.md` and the `init` runbook at the time RFC-33b lands.

## Relationship to RFC-28 / RFC-32 / RFC-33a

| Concern | RFC-28 | RFC-32 | RFC-33a | RFC-33b |
| --- | --- | --- | --- | --- |
| `LintFinding` envelope | Defines | Produces | Adds `ignored` to `status`; adds `disposition` with `directive?` | Adds `new` / `baselined` to `status`; adds `disposition.baseline?` |
| Fingerprint algorithm | Defines | Computes | Consumes (join key) | Consumes (join key) |
| `WorkspaceModel` | — | Defines + extracts | Adds `ignore_directive` fact | (no further change) |
| `specrun lint` core scan | — | Defines | Adds directive post-pass | Adds baseline mode + last-run persistence |
| Baseline file shape | — | — | — | Defines |
| Ignore directive grammar | — | — | Defines | Consumes (for diff `ignored[]` bucket) |
| Diff between runs | — | — | — | Defines |
| `EventKind` taxonomy | — | — | Adds `lint-completed` | Populates `baseline_present` truthfully; adds `counts.{new, baselined}` to the payload |
| CI blocking semantics | — | — | `open` blocks by default | `open | new` block by default |

## Alternatives considered

**A separate `acknowledgements/` directory of one file per accepted finding.** Rejected. Higher review cost in PRs, no fingerprint-based dedupe, harder to diff. The single-file baseline trades human readability of individual entries for byte-stable diffs and a one-shot operator workflow.

**Per-finding journal events.** Rejected. The journal is event-shape, not state-shape. Per-finding history belongs in `.specify/lint/last.json` and (eventually) RM-14's structured event sinks; the journal carries one summary event per scan.

**SARIF baseline interop.** Rejected for v1. The SARIF baseline mechanism is closely tied to its own envelope; mapping is feasible once Specify ships a SARIF export adapter, but inheriting SARIF semantics now would lock in choices RFC-28 deliberately avoided.

**Auto-clearing baseline entries when their fingerprint disappears.** Rejected. The diff verb reports stale entries; pruning is operator-driven via `specrun lint baseline drop` or manual edit + re-write. Auto-clear would erode the "one writer per file" principle (RFC-33a principle 6).

**Fold RFC-33b back into RFC-33a as a phase split.** Rejected. The earlier combined RFC-33 spanned ~410 lines and mixed an active surface (ignore directives, telemetry) with a deferred one (baseline, diff). Splitting along the active/deferred line keeps each document focused, matches the precedent set by RFC-3a / RFC-3b, and lets RFC-33b's trigger conditions and CLI surface stay together without being scattered across phase headings.

## Non-Goals

- PR comment rendering, dashboard aggregation, or any presentation layer beyond CLI output.
- Multi-baseline merging strategies (other than the per-target scoping rule above).
- Ignoring model-assisted findings before RFC-32 wires LLM producers into the deterministic core. Baseline applies to deterministic findings only.
- Workflow lifecycle transitions in any form (RFC-32 principle 7 carries forward).

## Open Questions

The three operational questions stay open until the RFC-33b implementation PR has a real surface to validate against:

1. Should `false-positive` directives count toward a separate budget that emits a `UNI-*` finding when exceeded? Current preference: out of scope for v1; revisit if dashboards report unbounded growth.
2. Should `specrun lint baseline drop` require `--yes` always or only in non-TTY environments? Current preference: always — the file is policy and dropping it changes CI behaviour for the next scan.
3. Should `last.json` be checked into source control? Current preference: no — it changes every run and dominates PR diffs. Operators wanting history should consume `lint-completed` events.

Q1, Q2, and Q4 from earlier drafts have been resolved into normative decisions D10, D11, and D12 (the last lives in [RFC-33a](../rfc-33a-ignore-directives.md)) respectively.

## References

- [RFC-28: Engineering Standards — Codex Contract and Findings](../done/rfc-28-standards-contract.md)
- [RFC-32: Engineering Standards — Deterministic Enforcement](../done/rfc-32-standards-enforcement.md)
- [RFC-33a: Standards Ignore Directives](../rfc-33a-ignore-directives.md) — companion RFC, active; provides the wire-shape decisions consumed here
- [Standards layer (explanation)](../../docs/explanation/standards-layer.md)
- [Specify Roadmap — RM-10](../roadmap.md#rm-10-ci-native-standards-enforcement)
- [Specify Roadmap — RM-14](../roadmap.md#rm-14-local-structured-workflow-events)
- [`crates/workflow/src/journal.rs`](https://github.com/augentic/specify-cli/blob/main/crates/workflow/src/journal.rs) — closed `EventKind` taxonomy
