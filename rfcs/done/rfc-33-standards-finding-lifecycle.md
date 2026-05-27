# RFC-33: Standards Finding Lifecycle

> Status: Accepted · Depends: [RFC-28](rfc-28-standards-contract.md), [RFC-32](rfc-32-standards-enforcement.md) · Enables: [roadmap RM-10](../roadmap.md#rm-10-ci-native-standards-enforcement), [roadmap RM-14](../roadmap.md#rm-14-local-structured-workflow-events)

## Abstract

[RFC-28](rfc-28-standards-contract.md) defines the `ReviewFinding` wire shape, fingerprint algorithm, and severity enum. [RFC-32](rfc-32-standards-enforcement.md) defines `WorkspaceModel`, the deterministic hint interpreter, and the `specrun review` scanner that emits findings for a single scan. Neither RFC defines the **cross-run** lifecycle: how findings are accepted as baseline debt, suppressed in source with rationale, diffed between scans, or summarised as journal events.

Without that layer, the first scan on any non-trivial consumer project returns hundreds of legitimate findings, CI fails on noise, and the engine is disabled before it earns its keep. Every long-lived linter (Clippy, ESLint, Semgrep, GitHub code scanning) eventually grows the same two surfaces: a baseline acknowledgement file for mass debt, and an in-source suppression marker for intentional exceptions with rationale. This RFC ships those surfaces inside the Specify contracts.

Concretely:

1. **Baseline file** — `.specify/review/baseline.json`, indexed by RFC-28 `fingerprint`. `specrun review` runs in baseline mode by default when the file is present; only findings outside the baseline block CI.
2. **In-source suppression markers** — a single `specify-ignore: <RULE-ID> — <rationale>` grammar the `WorkspaceModel` indexer recognises across known comment styles, surfaced as a `suppression_marker` fact and consumed by the hint interpreter at finding-emission time.
3. **Last-run persistence** — `.specify/review/last.json` carries the full envelope from the most recent run so `specrun review baseline diff` answers "what changed since last scan" without re-scanning.
4. **Finding status taxonomy** — the existing `status` field on `ReviewFinding` widens to `open | new | baselined | suppressed | fixed | accepted | false-positive`, with `disposition` carrying the source of the decision (baseline entry, marker location, operator action).
5. **Journal event** — `review-completed` added to the closed `EventKind` taxonomy in `crates/domain/src/journal.rs`, joining RM-14 telemetry without requiring RM-14 to ship first.

This RFC adds no lifecycle authority. Suppressed and baselined findings still never transition plan entries, slices, or changes. Baseline mode changes which findings count as CI blockers; it does not change the meaning of a finding.

## Motivation

`specrun review` and `specdev check --format json` will be the first surfaces in the Specify stack that operators can choose to either ignore or comply with. The cost of ignoring is high (CI loses signal); the cost of complying without staging is higher still (every rule must be cleared before the scanner can run on a real codebase).

The three pressure points after RFC-32 lands:

- **Mass adoption requires a staging mechanism.** A consumer project picking up RFC-32 on a codebase older than the rule set will see findings for code that was correct at the time it was written. Without baselines, the operator's only choices are to fix everything before turning on CI gates or to suppress entire rules globally. The first is impractical; the second discards the rule's value forever.
- **Intentional exceptions need to live with the code.** Some findings are legitimately wrong for one file or one line and should be tolerated in place. A baseline file is too coarse for that — it survives across files but loses context. The industry-standard answer is an in-source comment marker; the open design choice is its grammar and what it carries.
- **CI dashboards need diffs, not absolute counts.** "Project X has 412 findings" is not actionable. "PR Y introduced 3 new findings and fixed 1" is. Stable fingerprints are already there per RFC-28; what is missing is somewhere to compare them against.

[RFC-32 §"Stability"](rfc-32-standards-enforcement.md) deliberately stops at "one scan, byte-stable order." The cross-run concern is a different layer, and RFC-32's own principles (2: findings are wire format; 7: no lifecycle authority in review) hold this RFC to the same boundary.

### What this RFC does not repeat

- **RFC-28** owns the `ReviewFinding` envelope, the fingerprint algorithm, the severity enum, and the closed `rule-id` namespace. RFC-33 widens the existing optional `status` field's value set and adds one optional `disposition` field; the fingerprint, envelope shape, and severity are unchanged.
- **RFC-32** owns `WorkspaceModel`, the hint interpreter, and `specrun review`'s single-scan behaviour. RFC-33 adds one fact family (`suppression_marker`) to the model and one optional baseline-loading step to the scanner, both behind the same `scan_profile: consumer` flag.
- **RFC-14 / RM-14** owns workflow telemetry. RFC-33 adds one event kind; it does not define a telemetry sink, query tool, or aggregation surface.

## Principles

1. **The fingerprint is the join key.** Baseline entries, suppression matches, last-run diffs, and journal payloads all key by RFC-28 `fingerprint`. RFC-33 mints no new identifier.
2. **Baseline is opt-in but on-by-default once present.** The first scan after `specrun review baseline write` runs in baseline mode without a flag; explicit `--no-baseline` is required to ignore.
3. **Suppression requires rationale.** Every in-source marker must name a rule id and carry a non-empty rationale. Markers without rationale are themselves findings.
4. **Markers do not aggregate.** One marker covers one rule on one location; no file-level or directory-level suppression syntax. Mass adoption is the baseline file's job, not the marker's.
5. **Status is post-hoc, never blocking by itself.** A scanner emits findings with `status: open`. The baseline/marker post-pass may demote them to `baselined` or `suppressed`. CI policy decides whether `status != open` means "ignore" or "report-only."
6. **One writer per file.** `.specify/review/baseline.json` is only written by `specrun review baseline write`. `.specify/review/last.json` is only written by `specrun review`. No skill, no agent, no other CLI verb edits either file.
7. **No lifecycle authority in review.** RFC-32's principle 7 holds. Baseline and suppression state is presentation; it never transitions plans, slices, or changes.

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Baseline file** | A scan-profile-scoped baseline lives at `.specify/review/baseline.json`. | New `schemas/review/baseline.schema.json`; new `Baseline` DTO in `specify-domain`; `specrun review` loads and matches by `fingerprint`. |
| **D2 Baseline mode default** | When a baseline file matching the active scope exists under `.specify/review/`, `specrun review` runs in baseline mode unless `--no-baseline` is passed. Per-target files (`baseline.<target>.json`) override the project-wide `baseline.json` only when `--target <name>` is active; nearest-scope wins. `--baseline <path>` is an explicit override that bypasses the layering rule. | Two new flags on the `specrun review` clap surface (`--no-baseline`, `--baseline <path>`); one new branch in the scanner pipeline; selection table pinned in §"Baseline file". |
| **D3 Suppression marker grammar** | Single grammar `specify-ignore: <RULE-ID> — <rationale>` recognised inside known comment markers (`//`, `#`, `<!-- -->`, `--`, `/* */`). Marker applies to the next non-blank, non-comment line. | New `suppression_marker` fact in `WorkspaceModel`; new extractor in `review/index/`; new `disposition.marker` payload in emitted findings. |
| **D4 Marker-without-rationale is a finding** | An unrationaled marker emits `UNI-022` (`suppression-marker-missing-rationale`); a marker whose `<RULE-ID>` does not match any finding on its target line emits `UNI-023` (`suppression-marker-orphan`). | Add two `UNI-*` rules under `adapters/shared/codex/universal/` with `regex` deterministic hints; first hints whose evidence comes from the indexer's own `suppression_marker` facts. |
| **D5 Status enum widened** | `ReviewFinding.status` adds `new`, `baselined`, `suppressed` to the existing `open | fixed | accepted | false-positive`. | Schema enum update in `schemas/review/finding.schema.json`; producer-local field already excluded from the fingerprint per RFC-28 §"Fingerprint algorithm" — no fingerprint change. |
| **D6 Disposition object** | New optional `disposition: { source, baseline?, marker?, since? }` on `ReviewFinding`. | One new field in the schema; one new struct in `specify-domain`; populated only when `status != open`. |
| **D7 Last-run persistence** | `.specify/review/last.json` holds the previous run's envelope verbatim. | One additional write at scanner exit; one new schema `schemas/review/run.schema.json` (same shape as live emission). |
| **D8 Journal event** | New `review-completed` variant on the closed `EventKind` taxonomy. | One new variant in `crates/domain/src/journal.rs`; payload counts per-status; no per-finding detail to keep journal lines bounded. |
| **D9 Diff verb** | `specrun review baseline diff` reports `new[]`, `fixed[]`, `unchanged[]`, `suppressed[]`, `baselined[]` against `last.json`. | One new subcommand; pure function over two envelopes plus the baseline; no scan side effects. |
| **D10 Baseline omits rule body** | Baseline entries store only `(fingerprint, rule_id, path, line, kind, rationale, recorded_at)`. They never embed hint kind, regex pattern, or any other rule-body slice; interpreter drift surfaces through `evidence-payload` changes already encoded in the fingerprint. | `schemas/review/baseline.schema.json` pins the closed entry shape; no duplicate of `specrun codex export` output; baseline rewrites are fingerprint-driven, not rule-edit-driven. |
| **D11 `baseline write` reads `last.json`** | `specrun review baseline write` reads the most recent run from `.specify/review/last.json` and refuses if the file is missing or stale (mtime older than the most recent file mtime under any scoped artifact path). It does not re-scan unless `--rescan` is passed, in which case it runs a scan inline and writes the resulting envelope verbatim before persisting the baseline. | Closed input contract: scanner is the only writer of `last.json`; baseline write is a pure transformation of an already-reviewed envelope; `--rescan` flag covers the rare "fresh capture" path without forking the contract. |
| **D12 Marker rationale stays free-form** | The grammar is `specify-ignore: <RULE-ID> — <rationale>`. No structured qualifiers (`expires:`, `owner:`, `ticket:`); rationale is opaque text ≥ 16 characters. Structured policy lives in codex rules and baseline metadata, not in source comments. | `regex` hint on `UNI-022` is two-token (`<RULE-ID>` + rest); no qualifier parsing in the indexer; downstream tools render rationale as a single string field. |
| **D13 First-party rule files** | The two new universal codex rules are filed at fixed paths so step 7 of the implementation plan has concrete targets. | `adapters/shared/codex/universal/suppression-marker-missing-rationale.md` (id `UNI-022`) and `adapters/shared/codex/universal/suppression-marker-orphan.md` (id `UNI-023`). |

### Finding status taxonomy

| Value | Set by | Meaning |
| --- | --- | --- |
| `open` | scanner (raw) | Default for a freshly emitted finding before post-passes run. |
| `new` | baseline pass | Finding's fingerprint is not in the baseline. CI blocks on these. |
| `baselined` | baseline pass | Finding's fingerprint is in the baseline. Reported but does not block. |
| `suppressed` | marker pass | Finding's location matches a `specify-ignore` marker for its `rule-id`. Reported with `disposition.marker` filled in. |
| `fixed` | diff verb | Fingerprint present in `last.json`, absent from current scan. Synthetic — emitted only by `specrun review baseline diff`. |
| `accepted` | operator | Set by `specrun review baseline write --rationale <...>` for entries explicitly accepted, distinguished from those inherited at first capture. |
| `false-positive` | operator or marker | Marker with rationale prefixed `false-positive:` or operator baseline entry with `kind: false-positive`. Reported separately in dashboards. |

`open` and `new` are the only values that can be CI-blocking by default. `baselined`, `suppressed`, `accepted`, `false-positive`, `fixed` are presentation-only.

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
- `kind` is one of `accepted | false-positive`. `accepted` is the default for `specrun review baseline write`; `false-positive` requires `--false-positive` and a non-empty `--rationale`.
- `path`, `line`, and `rule_id` are denormalised from the matched finding for human review; the join key is `fingerprint` only.
- A baseline matches a finding when fingerprints are byte-equal. `path` / `line` drift is invisible because RFC-28's fingerprint excludes producer-local fields and includes `evidence-payload`, which captures the underlying code change.

Multiple baselines are supported by suffixing scope: `.specify/review/baseline.<target>.json` (e.g. `baseline.omnia.json`). The unsuffixed file is the project-wide baseline; suffixed files override per-target during target-scoped scans. Layering rule: nearest-scope wins.

Selection matrix (per D2):

| Active scope | Files present | File matched against scan |
| --- | --- | --- |
| `specrun review` (no `--target`) | `baseline.json` only | `baseline.json` |
| `specrun review` (no `--target`) | `baseline.json` + `baseline.<target>.json` | `baseline.json` only — target-suffixed files are ignored when no target is active |
| `specrun review --target omnia` | `baseline.json` only | `baseline.json` |
| `specrun review --target omnia` | `baseline.json` + `baseline.omnia.json` | `baseline.omnia.json` only — per-target supersedes project-wide |

Override flags:

- `--baseline <path>` selects a specific file regardless of suffix; it bypasses the matrix entirely and is intended for CI dry-runs and ad-hoc comparisons.
- `--no-baseline` defeats every baseline file regardless of suffix; the run reports all findings as `open`.

### Suppression markers

Grammar (one definition, applied across comment styles):

```text
specify-ignore: <RULE-ID> — <rationale>
```

- `<RULE-ID>` is a closed RFC-28 rule id (e.g. `UNI-014`, `OMNIA-021`).
- `—` is an em-dash *or* the two-character sequence `--` (authoring tolerance).
- `<rationale>` is non-empty, free-form text. Recommended ≥ 16 characters.
- One rule per marker. Two adjacent markers on consecutive lines compose: each applies to the same next non-blank, non-comment line.

Comment-style recognition (a closed list, owned by the indexer):

| Language family | Marker syntax accepted |
| --- | --- |
| C-family (Rust, JS, Go, TS, Swift, Java, C, C++) | `// specify-ignore: …`, `/* specify-ignore: … */` |
| Shell, Python, YAML, TOML | `# specify-ignore: …` |
| HTML, Markdown, XML | `<!-- specify-ignore: … -->` |
| SQL, Lua | `-- specify-ignore: …` |

Scope rules:

- A marker applies to the **next non-blank, non-comment line**. Inline `// specify-ignore: ...` placed at end-of-line on the same line as code applies to that same line.
- File-scoped and block-scoped variants are explicitly **not in v1**. Mass adoption is the baseline file's job (principle 4).
- A marker whose `<RULE-ID>` does not match any finding on its target line emits `UNI-023` (`suppression-marker-orphan`, new shared codex rule). Operators see dead markers in the same envelope as everything else.

WorkspaceModel addition (one new fact family):

```text
suppression_marker  { path, line, rule_id, rationale, target_line }
```

The indexer extracts markers once per scan; the hint interpreter consumes them at finding-emission time and stamps `status: suppressed` plus `disposition.marker` on matches.

### Last-run persistence

`.specify/review/last.json` is the verbatim envelope from the most recent `specrun review` run (any flags, any scope). One file per project. It is overwritten on every run.

Diff semantics:

- `new[]` — fingerprints in current scan, not in `last.json`.
- `fixed[]` — fingerprints in `last.json`, not in current scan.
- `unchanged[]` — fingerprints in both.
- `suppressed[]`, `baselined[]` — fingerprints in current scan with the matching status.

The diff verb is a pure function over `last.json`, the current scan, and the baseline. It never re-scans and never writes either file.

### Journal event

Single new variant on the closed `EventKind` taxonomy in `crates/domain/src/journal.rs`:

```text
review-completed
```

Payload:

```json
{
  "scope": { "target": "omnia", "slice": null, "artifact": null },
  "duration_ms": 824,
  "counts": {
    "open": 12,
    "new": 3,
    "baselined": 89,
    "suppressed": 4,
    "false_positive": 0
  },
  "baseline_present": true,
  "exit_code": 2
}
```

No per-finding detail in the journal. Operators wanting per-finding history read `.specify/review/last.json` (current) or build their own log on top of `specrun events tail` once RM-14 ships.

### CLI surface

```bash
specrun review                                  # scans; baseline mode if scope-matching file present
specrun review --no-baseline                    # ignore every baseline file regardless of suffix
specrun review --baseline path/to/file.json     # explicit override; bypasses the §"Baseline file" matrix
specrun review baseline write                   # consume .specify/review/last.json and write the baseline
specrun review baseline write --rescan          # scan inline, persist last.json, then write the baseline
specrun review baseline write --rationale "Legacy import; tracked in slice X."
specrun review baseline write --false-positive --rationale "Match in vendored fixture."
specrun review baseline diff                    # compare current scan to last.json + baseline
specrun review baseline diff <fingerprint>      # focus the diff on one finding
specrun review baseline drop                    # remove the baseline file (journals the action)
```

Behavioural notes:

- `specrun review baseline write` reads `.specify/review/last.json` rather than re-scanning (D11). It refuses with `review-baseline-write-stale` when `last.json` is missing, or when its mtime is older than the most recent file mtime under any scoped artifact path. Operators recover by re-running `specrun review` or by passing `--rescan` to capture and persist a fresh envelope inline.
- `specrun review baseline write` requires `--yes` in non-TTY environments to confirm the silent acceptance of every current finding. In TTY mode it prints a count summary and asks for confirmation.
- `specrun review baseline write --append <fingerprint>` adds one entry to an existing baseline without reading `last.json`; useful when an operator wants to accept a known finding by fingerprint alone.
- `specrun review baseline drop` emits a `review-completed` event with `baseline_present: false` and a synthetic count of zero, so the journal records the policy change.

### Schema changes

Two new schema files, one extended:

| File | Status | Owner |
| --- | --- | --- |
| `schemas/review/finding.schema.json` | Extended | RFC-28 (this RFC extends the `status` enum and adds the optional `disposition` field) |
| `schemas/review/baseline.schema.json` | New | RFC-33 |
| `schemas/review/run.schema.json` | New | RFC-33 (matches the live envelope shape) |

Backwards compatibility:

- The fingerprint algorithm does **not** change. RFC-28 already excludes `status` from the fingerprint.
- The added `status` values (`new`, `baselined`, `suppressed`) are additive; consumers that only handle `open | fixed | accepted | false-positive` see `null` if they reject unknown values, but the schema enum widens cleanly.
- The new optional `disposition` field is unset on raw scanner output, so RFC-28's "raw scanner" producer contract is unchanged.

### Relationship to RFC-28 / RFC-32

| Concern | RFC-28 | RFC-32 | RFC-33 |
| --- | --- | --- | --- |
| `ReviewFinding` envelope | Defines | Produces | Extends `status`; adds `disposition` |
| Fingerprint algorithm | Defines | Computes | Consumes (join key) |
| `WorkspaceModel` | — | Defines + extracts | Adds `suppression_marker` fact |
| `specrun review` core scan | — | Defines | Adds baseline mode and last-run persistence |
| Baseline file shape | — | — | Defines |
| Suppression marker grammar | — | — | Defines |
| Diff between runs | — | — | Defines |
| `EventKind` taxonomy | — | — | Adds `review-completed` |
| CI blocking semantics | — | — | Defines (`open | new` block by default) |

## Implementation plan

1. **Schemas.** Extend `schemas/review/finding.schema.json` (status enum, `disposition`); add `schemas/review/baseline.schema.json` and `schemas/review/run.schema.json`. Golden test: fingerprint stability across the enum widening, asserted against the RFC-28 Phase 2 fixtures.
2. **Domain types.** Add `Baseline`, `BaselineEntry`, `Disposition`, `ReviewRun` DTOs to `specify-domain`. Reuse RFC-28's canonical-JSON helper for stable serialisation.
3. **Indexer fact.** Add `suppression_marker` extractor under `crates/domain/src/review/index/`. Honour the closed comment-style list; ignore everything else without falling back to heuristics.
4. **Scanner pipeline.** Insert baseline + marker passes between hint evaluation and envelope emission. Order: marker → baseline → status assignment → ordering.
5. **CLI surface.** Add `specrun review baseline {write, write --append, write --rescan, drop, diff}` subcommands and the `--no-baseline` / `--baseline <path>` flags on `specrun review`. Enforce the §"Baseline file" selection matrix, the `last.json` staleness check from D11 (`review-baseline-write-stale`), and confirm-or-`--yes` discipline on every write.
6. **Journal.** Add the `review-completed` variant and wire it into the existing emission path. One new event-shape fixture.
7. **First-party rules.** Author the two `UNI-*` codex rules pinned by D13: `adapters/shared/codex/universal/suppression-marker-missing-rationale.md` (id `UNI-022`) and `adapters/shared/codex/universal/suppression-marker-orphan.md` (id `UNI-023`), each with a `regex` deterministic hint sourced from the indexer's own `suppression_marker` facts. These are the first hints that exist *because* of the indexer's own facts; resolver failures degrade per the §"Migration" graceful-degradation rule.
8. **Acceptance.** Golden tests: scan with baseline absent → all `open`; scan with baseline present → mix of `baselined` and `new`; scan with marker → `suppressed`; diff across two scans → stable `new[]` / `fixed[]` / `unchanged[]`.

## Migration

**For operator projects:** Additive. Without a baseline file or markers, behaviour is identical to RFC-32 Phase 2. To adopt baseline mode, run `specrun review` (which writes `.specify/review/last.json`) then `specrun review baseline write`, review the file in PR, and commit. Markers may land alongside the code they apply to in the same PR that introduces the protected change.

**For adapter authors:** No required action. Adapter codex rules may add markers in their own examples; the new shared `UNI-022` / `UNI-023` rules apply to consumer-project code, not to adapter source.

**For framework contributors:** No required action in Phase 2. If RFC-28 Phase 3 (`specdev check --format json`) is in scope by the time this RFC ships, the same `status`/`disposition` field set applies to framework findings without code changes on the `specdev` side.

**For CLI maintainers:** The baseline and run files live under `.specify/review/` — a new sub-tree, distinct from `.specify/cache/`, `.specify/slices/`, and `.specify/archive/`. Document the directory in `docs/reference/directory-layout.md` and the `init` runbook.

### Graceful degradation when the universal codex tree is absent

`UNI-022` and `UNI-023` live in `adapters/shared/codex/universal/` and reach the scanner via the RFC-28 codex resolver. On consumer projects that have not yet picked up the shared codex tree (the [RM-10 "Distribution follow-up"](../roadmap.md#rm-10-ci-native-standards-enforcement) gap), the resolver simply does not produce those two rules, and the scanner emits neither `UNI-022` nor `UNI-023` findings. Everything else specified by this RFC remains functional:

- The `suppression_marker` indexer fact (D3) is extracted from source unconditionally; it does not depend on the codex resolution outcome.
- `disposition.marker` is stamped on findings that match a marker for their `rule-id`, again independent of whether the universal tree resolved.
- Baseline mode (D2), last-run persistence (D7), the diff verb (D9), and the `review-completed` journal event (D8) operate solely on the local envelope and the scoped baseline file; none of them depend on `UNI-022` / `UNI-023` being resolvable.

The only operator-visible regression in this configuration is that unrationaled markers and orphan markers slip through silently. The fix is to either pass `--codex-root` to `specrun review` until the distribution follow-up lands, or to vendor the shared tree into the consumer project's `.specify/cache/codex/` location read by `resolve_codex_root` (see [`src/runtime/commands/review/run.rs`](https://github.com/augentic/specify-cli/blob/main/src/runtime/commands/review/run.rs)).

This degradation is intentional: it keeps RFC-33 deliverable independently of the RM-10 distribution work, at the cost of a documented caveat for consumer projects that adopt baseline + marker tracking before the shared tree is colocated.

## Alternatives considered

**File-level or directory-level suppression markers.** Rejected for v1. Encourages "lint shopping" where an inconvenient rule is disabled wholesale rather than baselined with context. The baseline file already solves the mass-adoption case, and it does so with explicit operator review at write time.

**A separate `acknowledgements/` directory of one file per accepted finding.** Rejected. Higher review cost in PRs, no fingerprint-based dedupe, harder to diff. The single-file baseline trades human readability of individual entries for byte-stable diffs and a one-shot operator workflow.

**Use existing `status: accepted` for both baseline and marker matches.** Rejected. Operators need to distinguish "we acknowledged this in the baseline" from "we wrote a comment next to the code explaining why." Conflating them loses signal in dashboards and PR comments.

**Per-finding journal events.** Rejected. The journal is event-shape, not state-shape. Per-finding history belongs in `.specify/review/last.json` and (eventually) RM-14's structured event sinks; the journal carries one summary event per scan.

**Sarif baseline interop.** Rejected for v1. The SARIF baseline mechanism is closely tied to its own envelope; mapping is feasible once Specify ships a SARIF export adapter, but inheriting SARIF semantics now would lock in choices RFC-28 deliberately avoided.

## Non-Goals

- PR comment rendering, dashboard aggregation, or any presentation layer beyond CLI output.
- Multi-baseline merging strategies (other than the per-target scoping rule above).
- Suppression markers that span ranges, files, or directories.
- Auto-clearing baseline entries when their fingerprint disappears (the diff verb reports them; pruning is operator-driven via `specrun review baseline drop` or manual edit + re-write).
- Suppression of model-assisted findings before RFC-32 wires LLM producers into the deterministic core. Markers apply to deterministic findings only in v1.
- Workflow lifecycle transitions in any form (RFC-32 principle 7 carries forward).

## Open Questions

Q1 (baseline body), Q2 (`baseline write` input), and Q4 (rationale qualifiers) from earlier drafts have been resolved into normative decisions D10, D11, and D12 respectively. The three remaining operational questions stay open until the implementation PR has a real surface to validate against:

1. Should `false-positive` markers count toward a separate budget that emits a `UNI-*` finding when exceeded? Current preference: out of scope for v1; revisit if dashboards report unbounded growth.
2. Should `specrun review baseline drop` require `--yes` always or only in non-TTY environments? Current preference: always — the file is policy and dropping it changes CI behaviour for the next scan.
3. Should `last.json` be checked into source control? Current preference: no — it changes every run and dominates PR diffs. Operators wanting history should consume `review-completed` events.

## References

- [RFC-28: Engineering Standards — Codex Contract and Findings](rfc-28-standards-contract.md)
- [RFC-32: Engineering Standards — Deterministic Enforcement](rfc-32-standards-enforcement.md)
- [Standards layer (explanation)](../../docs/explanation/standards-layer.md)
- [Specify Roadmap — RM-10](../roadmap.md#rm-10-ci-native-standards-enforcement)
- [Specify Roadmap — RM-14](../roadmap.md#rm-14-local-structured-workflow-events)
- [`crates/domain/src/journal.rs`](https://github.com/augentic/specify-cli/blob/main/crates/domain/src/journal.rs) — closed `EventKind` taxonomy
