# RFC-33b: Standards Baseline

> Status: Deferred · Depends (all landed): the [Diagnostic substrate](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate) (RFC-28), the [deterministic consumer scanner](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema) (RFC-32), and the [lint-finding lifecycle](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lint-finding-status-disposition-and-exit) (RFC-33a) · Enables: [roadmap RM-14](../roadmap.md#rm-14-local-structured-workflow-events)

## Abstract

The shipped lint lifecycle (RFC-33a) delivers in-source ignore directives, the `ignored` finding status, the `disposition` field with its `directive` sub-field, and the `lint-completed` journal event. It does not deliver the **cross-run** lifecycle: how a project acknowledges a body of legitimate findings as baseline debt, how scans diff against prior runs, and how operators stage remediation across releases.

This RFC pre-designs that surface so it can land against a settled contract once one of the trigger conditions in §"Trigger conditions" is met. While deferred, no baseline code, schema file, CLI verb, or filesystem path under `.specify/lint/` ships. RFC-33b extends the live wire shape additively: today `specify-diagnostics` carries `status ∈ { open, ignored, fixed, accepted, false-positive }` and `disposition.source ∈ { directive }` (the schema comment already reserves a future baseline source). RFC-33b adds `new` and `baselined` to `status` and `baseline` to `disposition.source`, plus the optional `disposition.baseline` payload.

Concretely, RFC-33b adds:

1. **Baseline file** — `.specify/lint/baseline.json`, indexed by the `specify-diagnostics` `fingerprint`. `specify lint project` runs in baseline mode by default when the file is present; only findings outside the baseline block CI.
2. **Last-run persistence** — `.specify/lint/last.json` carries the full `DiagnosticReport` from the most recent run so `specify lint project baseline diff` answers "what changed since last scan" without re-scanning.
3. **Diff verb** — `specify lint project baseline diff` reports `new[] / fixed[] / unchanged[] / ignored[] / baselined[]` as a pure function over `last.json`, the current scan, and the baseline.
4. **CLI surface** — `specify lint project baseline { write, drop, diff }` subcommands plus `--no-baseline` / `--baseline <path>` flags on `specify lint project`.
5. **Status enum widening** — widens `schemas/diagnostics/diagnostic.schema.json` to add `new` and `baselined` to `status` and `baseline` to `disposition.source`, plus the optional `disposition.baseline` sub-field. Additive; the fingerprint algorithm is unchanged.

This RFC adds no lifecycle authority. Baselined findings never transition plan entries, slices, or changes. Baseline mode changes which findings count as CI blockers; it does not change the meaning of a finding.

## Motivation

RFC-33a's ignore directives address intentional, code-local exceptions. Two pressures remain, and both are conditional — neither bites under a "resolve every finding before release" operator policy on a Specify-native codebase:

- **Mass adoption needs a staging mechanism.** A consumer project picking up the shared rule set on a codebase older than the rules will see findings for code that was correct when written. Without baselines the only choices are fix-everything-first (impractical) or disable-the-rule-globally (discards its value forever).
- **CI dashboards need diffs, not absolute counts.** "Project X has 412 findings" is not actionable; "PR Y introduced 3 new findings and fixed 1" is. Stable fingerprints already exist (RFC-28); what is missing is somewhere to compare them against.

### Trigger conditions

Any one of the following promotes RFC-33b from deferred to active:

1. **Retroactive rule pressure.** A new `UNI-*`, `OMNIA-*`, `VECTIS-*`, or other shared rule lands that flags existing slice code with no immediate-fix path; the operator needs to acknowledge the debt while cleanup is tracked in plan entries.
2. **Pre-existing codebase intake.** A consumer project onboards code that did not flow through `/spec:build` — e.g. the TypeScript → Rust WASM reconstruction input. Day-one scans surface mass legitimate findings that cannot be cleared before CI is enabled.
3. **Operator policy change.** "Resolve every finding before release" stops being feasible at the team's cadence, and staged remediation across releases becomes the norm. At that point the `last.json` + diff signal becomes non-trivial.

Each trigger has a clean owner: (1) shared-rule resolver output deltas, (2) the source adapter onboarding the codebase (`typescript`, future `java`, …), (3) an explicit operator decision documented in the project's release runbook.

### What this RFC does not repeat

- **RFC-28** owns the `Diagnostic` envelope, the fingerprint algorithm, the severity enum, and the closed `rule-id` namespace (in `specify-diagnostics`). RFC-33b consumes the fingerprint as its join key; it makes no envelope changes beyond the additive enum widening.
- **RFC-32** owns `WorkspaceModel`, the hint interpreter, and `specify lint project`'s single-scan behaviour (in `specify-standards`). RFC-33b adds one optional baseline-loading step to the scanner pipeline.
- **RFC-33a** owns the ignore-directive grammar, the `ignored` status, the `disposition.directive` payload, and the `lint-completed` journal event. RFC-33b widens the schema again and supplies producers for the pre-existing `fixed` and `accepted` values.

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Baseline file** | A scan-profile-scoped baseline lives at `.specify/lint/baseline.json`. | New `schemas/lint/baseline.schema.json`; new `Baseline` DTO in `specify-standards`; `specify lint project` loads and matches by `fingerprint`. |
| **D2 Baseline mode default** | When a baseline file matching the active scope exists, `specify lint project` runs in baseline mode unless `--no-baseline` is passed. Per-target files (`baseline.<target>.json`) override the project-wide `baseline.json` only when `--target <name>` is active; nearest-scope wins. `--baseline <path>` bypasses the layering rule. | Two new flags (`--no-baseline`, `--baseline <path>`); one new branch in the scanner pipeline; selection matrix pinned below. |
| **D7 Last-run persistence** | `.specify/lint/last.json` holds the previous run's `DiagnosticReport` verbatim. | One additional write at scanner exit; new `schemas/lint/run.schema.json` (matching the live `diagnostic-report.schema.json` shape). |
| **D9 Diff verb** | `specify lint project baseline diff` reports `new[]`, `fixed[]`, `unchanged[]`, `ignored[]`, `baselined[]` against `last.json`. | One new subcommand; pure function over two reports plus the baseline; no scan side effects. |
| **D10 Baseline omits rule body** | Baseline entries store only `(fingerprint, rule-id, path, line, kind, rationale, recorded-at)` — never hint kind, regex, or other rule-body slices; interpreter drift surfaces through the `evidence` payload already folded into the fingerprint. | `schemas/lint/baseline.schema.json` pins the closed entry shape; baseline rewrites are fingerprint-driven, not rule-edit-driven. |
| **D11 `baseline write` reads `last.json`** | `specify lint project baseline write` reads the most recent run from `.specify/lint/last.json` and refuses if it is missing or stale. `--rescan` runs a scan inline and persists a fresh report before writing the baseline. | Closed input contract: the scanner is the only writer of `last.json`; baseline write is a pure transformation of an already-approved report. |

The decision IDs are preserved from the original combined RFC-33; RFC-33a owns D3–D6, D8, D12, D13.

### Baseline file

```json
{
  "version": 1,
  "created-at": "2026-05-27T20:42:00Z",
  "scan-profile": "consumer",
  "scope": { "target": "omnia", "artifact": null },
  "entries": [
    {
      "fingerprint": "sha256:…",
      "rule-id": "UNI-014",
      "path": "crates/billing/src/config.rs",
      "line": 18,
      "recorded-at": "2026-05-27T20:42:00Z",
      "kind": "accepted",
      "rationale": "Legacy endpoint; tracked in plan slice billing-config-extraction."
    }
  ]
}
```

Required behaviour:

- Entries are sorted by `(rule-id, path, line, fingerprint)` for byte-stable diffs.
- `kind` is one of `accepted | false-positive`. `accepted` is the default for `baseline write`; `false-positive` requires `--false-positive` and a non-empty `--rationale`.
- `path`, `line`, and `rule-id` are denormalised from the matched finding for human review; the join key is `fingerprint` only.
- A baseline matches a finding when fingerprints are byte-equal. `path` / `line` drift is invisible because the fingerprint excludes producer-local fields and folds in the `evidence` payload, which captures the underlying code change.

Multiple baselines are supported by suffixing scope: `.specify/lint/baseline.<target>.json`. The unsuffixed file is project-wide; suffixed files override per-target during target-scoped scans. Nearest-scope wins:

| Active scope | Files present | File matched against scan |
| --- | --- | --- |
| `specify lint project` (no `--target`) | `baseline.json` only | `baseline.json` |
| `specify lint project` (no `--target`) | `baseline.json` + `baseline.<target>.json` | `baseline.json` only — target-suffixed files are ignored when no target is active |
| `specify lint project --target omnia` | `baseline.json` only | `baseline.json` |
| `specify lint project --target omnia` | `baseline.json` + `baseline.omnia.json` | `baseline.omnia.json` only — per-target supersedes project-wide |

Override flags: `--baseline <path>` selects a specific file regardless of suffix (CI dry-runs, ad-hoc comparisons); `--no-baseline` defeats every baseline file, reporting all findings as `open`.

### Last-run persistence

`.specify/lint/last.json` is the verbatim `DiagnosticReport` from the most recent `specify lint project` invocation (any flags, any scope). One file per project, overwritten on every run. Diff semantics:

- `new[]` — fingerprints in the current scan, not in `last.json`.
- `fixed[]` — fingerprints in `last.json`, not in the current scan.
- `unchanged[]` — fingerprints in both.
- `ignored[]`, `baselined[]` — fingerprints in the current scan carrying the matching status.

The diff verb is a pure function over `last.json`, the current scan, and the baseline. It never re-scans and never writes either file.

### CLI surface

```bash
specify lint project --no-baseline                 # ignore every baseline file regardless of suffix
specify lint project --baseline path/to/file.json  # explicit override; bypasses the selection matrix
specify lint project baseline write                # consume .specify/lint/last.json and write the baseline
specify lint project baseline write --rescan       # scan inline, persist last.json, then write the baseline
specify lint project baseline write --rationale "Legacy import; tracked in slice X."
specify lint project baseline write --false-positive --rationale "Match in vendored fixture."
specify lint project baseline diff                 # compare current scan to last.json + baseline
specify lint project baseline diff <fingerprint>   # focus the diff on one finding
specify lint project baseline drop                 # remove the baseline file (journals the action)
```

Behavioural notes:

- `baseline write` reads `.specify/lint/last.json` rather than re-scanning (D11). It refuses with `lint-baseline-write-stale` when `last.json` is missing or older than the most recent scoped artifact mtime; operators recover by re-running `specify lint project` or passing `--rescan`.
- `baseline write` requires `--yes` in non-TTY environments to confirm silent acceptance of every current finding. In TTY mode it prints a count summary and asks for confirmation.
- `baseline write --append <fingerprint>` adds one entry without reading `last.json`, for accepting a known finding by fingerprint alone.
- `baseline drop` emits a `lint-completed` event with `baseline-present: false`, so the journal records the policy change.

### Schema changes

| File | Status | Owner |
| --- | --- | --- |
| `schemas/lint/baseline.schema.json` | New | RFC-33b |
| `schemas/lint/run.schema.json` | New | RFC-33b (matches the `diagnostic-report.schema.json` shape) |

`schemas/diagnostics/diagnostic.schema.json` widens additively: `status` gains `new` and `baselined`, `disposition.source` gains `baseline`, and `disposition` gains the optional `baseline` payload. The fingerprint algorithm is unchanged.

### Implementation plan

1. **Schemas.** Add `schemas/lint/baseline.schema.json` and `schemas/lint/run.schema.json`; widen `diagnostic.schema.json` additively as above. Mirror the embedded constants in `specify-schema`.
2. **Standards-layer types.** Add `Baseline`, `BaselineEntry`, `ReviewRun` DTOs to `specify-standards`; reuse the `specify-diagnostics` canonical-JSON / fingerprint helpers.
3. **Scanner pipeline.** Insert the baseline pass after directive matching: hint evaluation → default `status: open` → directive validation/matching → baseline matching → ordering → report/render → status-aware exit.
4. **Last-run persistence.** Write `.specify/lint/last.json` at scanner exit, after report emission.
5. **CLI surface.** Add the `specify lint project baseline { write, write --append, write --rescan, drop, diff }` subcommands and the `--no-baseline` / `--baseline <path>` flags; enforce the selection matrix, the `lint-baseline-write-stale` check, and confirm-or-`--yes` discipline.
6. **Journal.** Populate `baseline-present` from the scan's actual baseline state; add `counts.{ new, baselined }` to the `lint-completed` payload.
7. **Acceptance.** Golden tests: baseline absent → all `open`; baseline present → mix of `baselined` and `new`; diff across two scans → stable buckets; per-target layering per the matrix.

### Migration

**Operators:** Additive. Without a baseline file, behaviour is identical to RFC-33a. To adopt baseline mode, run `specify lint project` (writes `last.json`), then `specify lint project baseline write`, review the file in PR, and commit.

**CLI maintainers:** The baseline and run files live under `.specify/lint/` — a new sub-tree, distinct from `.specify/cache/`, `.specify/slices/`, and `.specify/archive/`. Document it in `docs/reference/directory-layout.md` and the `init` runbook when RFC-33b lands.

## Relationship to RFC-28 / RFC-32 / RFC-33a

| Concern | RFC-28 | RFC-32 | RFC-33a | RFC-33b |
| --- | --- | --- | --- | --- |
| `Diagnostic` envelope | Defines | Produces | Adds `ignored` to `status`; adds `disposition.directive` | Adds `new` / `baselined`; adds `disposition.baseline` |
| Fingerprint algorithm | Defines | Computes | Consumes (join key) | Consumes (join key) |
| `WorkspaceModel` | — | Defines + extracts | Adds `ignore-directive` fact | (no further change) |
| `specify lint project` core scan | — | Defines | Adds directive post-pass | Adds baseline mode + last-run persistence |
| Baseline file shape | — | — | — | Defines |
| Diff between runs | — | — | — | Defines |
| `lint-completed` event | — | — | Adds | Populates `baseline-present`; adds `counts.{ new, baselined }` |
| CI blocking semantics | — | — | `open` blocks by default | `open \| new` block by default |

## Alternatives considered

- **A directory of one file per accepted finding.** Rejected: higher PR review cost, no fingerprint dedupe, harder to diff. The single-file baseline trades per-entry readability for byte-stable diffs and a one-shot operator workflow.
- **Per-finding journal events.** Rejected: the journal is event-shape, not state-shape. Per-finding history belongs in `last.json` and (eventually) RM-14's structured sinks; the journal carries one summary event per scan.
- **SARIF baseline interop.** Rejected for v1: SARIF's baseline mechanism is tied to its own envelope; mapping is feasible once Specify ships a SARIF export adapter but would inherit semantics RFC-28 deliberately avoided.
- **Auto-clearing stale baseline entries.** Rejected: the diff verb reports stale entries; pruning is operator-driven via `baseline drop` or manual edit + re-write. Auto-clear would erode "one writer per file".

## Non-Goals

- PR comment rendering, dashboard aggregation, or any presentation layer beyond CLI output.
- Multi-baseline merging beyond the per-target scoping rule above.
- Baselining model-assisted findings — baseline applies to deterministic findings only.
- Workflow lifecycle transitions in any form.

## Open Questions

1. Should `false-positive` directives count toward a separate budget that emits a `UNI-*` finding when exceeded? Current preference: out of scope for v1.
2. Should `baseline drop` require `--yes` always or only in non-TTY environments? Current preference: always — the file is policy and dropping it changes CI behaviour for the next scan.
3. Should `last.json` be checked into source control? Current preference: no — it changes every run and dominates PR diffs. Operators wanting history should consume `lint-completed` events.

## References

- [Diagnostic substrate](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate) (RFC-28) — the `Diagnostic` envelope and fingerprint, in `specify-diagnostics`.
- [Standards layer split](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema) (RFC-32) — `WorkspaceModel` and the consumer scanner, in `specify-standards`.
- [Lint finding lifecycle](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lint-finding-status-disposition-and-exit) (RFC-33a) — ignore directives, the `disposition` field, and the `lint-completed` event.
- [Standards layer (explanation)](../../docs/explanation/standards-layer.md)
- [Specify Roadmap — RM-14](../roadmap.md#rm-14-local-structured-workflow-events)
