# RFC-33a: Ignore Directives

> Status: Accepted · Depends: [RFC-28](done/rfc-28-standards-contract.md), [RFC-32](done/rfc-32-standards-enforcement.md) · Companion: [RFC-33b](future/rfc-33b-standards-baseline.md) (deferred) · Enables: [roadmap RM-10](roadmap.md#rm-10-ci-native-standards-enforcement), [roadmap RM-14](roadmap.md#rm-14-local-structured-workflow-events)

## Abstract

[RFC-28](done/rfc-28-standards-contract.md) defines the `LintFinding` wire shape, fingerprint algorithm, and severity enum. [RFC-32](done/rfc-32-standards-enforcement.md) defines `WorkspaceModel`, the deterministic hint interpreter, and the `specrun lint` scanner that emits findings for a single scan. Neither RFC defines the **per-line tolerance** layer: how operators acknowledge a single legitimate exception in source, what shape carries the rationale, and how scanner runs surface in telemetry.

Without that layer, the first scan on a codebase with even one false positive forces operators to either disable the rule globally or land speculative rule fixes. Every long-lived linter (Clippy, ESLint, Semgrep, GitHub code scanning) eventually grows an in-source ignore directive with rationale; this RFC ships the Specify equivalent inside the deterministic core.

This RFC delivers three additions:

1. **In-source ignore directives** — a single `specify-ignore: <RULE-ID> — <rationale>` grammar the `WorkspaceModel` indexer recognises across known comment styles, surfaced as an `ignore_directive` fact and consumed by a directive post-pass at finding-emission time.
2. **Finding status taxonomy and `disposition` field** — the existing `status` field on `LintFinding` widens to add `ignored` to RFC-28's `open` / `fixed` / `accepted` / `false-positive` set, with the new optional `disposition` field carrying the source of the decision. RFC-33a producers emit `open`, `ignored`, and `false-positive`. [RFC-33b](future/rfc-33b-standards-baseline.md) widens the enum a second time (adding `new` and `baselined`) and adds the optional `disposition.baseline` sub-field when it lands; both widenings are additive.
3. **Journal event** — `lint-completed` added to the closed `EventKind` taxonomy in `crates/domain/src/journal.rs`, joining RM-14 telemetry without requiring RM-14 to ship first.

The cross-run baseline + diff layer is split into [RFC-33b: Standards Baseline](future/rfc-33b-standards-baseline.md), which is deferred behind the trigger conditions named in its own §"Trigger conditions". RFC-33b extends RFC-33a's D5 / D6 wire-shape decisions additively: it widens the `status` enum to add `new` and `baselined`, and adds the optional `disposition.baseline` sub-field when it lands.

This RFC adds no lifecycle authority. Ignored findings never transition plan entries, slices, or changes.

## Motivation

`specrun lint` and `specdev lint --format json` will be the first surfaces in the Specify stack that operators can choose to either ignore or comply with. The cost of ignoring is high (CI loses signal); the cost of complying without an escape hatch is higher still (every false positive at the rule layer becomes a blocker until the rule itself is improved).

The pressure point this RFC addresses, and how soon it bites:

- **Intentional exceptions need to live with the code.** Some findings are legitimately wrong for one file or one line and should be tolerated in place. A baseline file (RFC-33b) is too coarse for that — it survives across files but loses context. The industry-standard answer is an in-source ignore directive; the open design choice is its grammar and what it carries. **This pressure point arrives on the first run of `specrun lint` against any non-trivial codebase, regardless of operator policy. RFC-33a addresses it.**

The two pressure points RFC-33a deliberately does not address, and why they live in [RFC-33b](future/rfc-33b-standards-baseline.md):

- **Mass adoption requires a staging mechanism.** A consumer project picking up RFC-32 on a codebase older than the rule set will see findings for code that was correct at the time it was written. This pressure point arrives only when (a) a consumer project predates the rule set, or (b) a new rule lands retroactively. Under a "resolve every finding before release" policy on a Specify-native codebase, neither condition holds today.
- **CI dashboards need diffs, not absolute counts.** "Project X has 412 findings" is not actionable. "PR Y introduced 3 new findings and fixed 1" is. Under fix-before-release every prior scan on `main` is clean, so the diff signal collapses to "every current finding is new."

Both pressure points are pre-designed in RFC-33b so the eventual implementation lands against a settled contract rather than freshly designed prose.

[RFC-32 §"Stability"](done/rfc-32-standards-enforcement.md) deliberately stops at "one scan, byte-stable order." Per-line ignoring and cross-run state are different layers; RFC-32's own principles (2: findings are wire format; 7: no lifecycle authority in review) hold both this RFC and RFC-33b to the same boundary.

### What this RFC does not repeat

- **RFC-28** owns the `LintFinding` envelope, the fingerprint algorithm, the severity enum, and the closed `rule-id` namespace. RFC-33a widens the existing optional `status` field's value set and adds one optional `disposition` field; the fingerprint, envelope shape, and severity are unchanged.
- **RFC-32** owns `WorkspaceModel`, the hint interpreter, and `specrun lint`'s single-scan behaviour. RFC-33a adds one fact family (`ignore_directive`) to the model, behind the same `scan_profile: consumer` flag.
- **RFC-33b** owns the baseline file, the last-run persistence, the diff verb, and the per-target selection matrix. RFC-33a establishes the wire-shape decisions (D5, D6) that RFC-33b consumes; RFC-33b does not re-define them.
- **RFC-14 / RM-14** owns workflow telemetry. RFC-33a adds one event kind; it does not define a telemetry sink, query tool, or aggregation surface.

## Principles

The principles below apply to both RFC-33a (this document) and [RFC-33b](future/rfc-33b-standards-baseline.md). RFC-33a implements (1), (3), (4), (5), and (7); RFC-33b picks up (2) and (6) when its surfaces land.

1. **The fingerprint is the join key.** Ignore-directive matches, baseline entries (RFC-33b), last-run diffs (RFC-33b), and journal payloads all key by RFC-28 `fingerprint`. No new identifier is minted.
2. **Baseline is opt-in but on-by-default once present.** ([RFC-33b](future/rfc-33b-standards-baseline.md).)
3. **Ignore directives require rationale.** Every in-source directive must name a rule id and carry a non-empty rationale. Directives without rationale are themselves findings.
4. **Directives do not aggregate.** One directive covers one rule on one location; no file-level or directory-level ignore syntax. Mass adoption is the baseline file's job ([RFC-33b](future/rfc-33b-standards-baseline.md)), not the directive's.
5. **Status is post-hoc, never blocking by itself.** A scanner emits findings with `status: open`. The directive post-pass may demote them to `ignored` (or `false-positive` for prefixed rationale). [RFC-33b](future/rfc-33b-standards-baseline.md) adds a baseline pass that demotes to `baselined`. CI policy decides whether `status != open` means "ignore" or "report-only."
6. **One writer per file.** ([RFC-33b](future/rfc-33b-standards-baseline.md), once the baseline file and `last.json` exist.)
7. **No lifecycle authority in review.** RFC-32's principle 7 holds. Ignored status is presentation; it never transitions plans, slices, or changes.

## Design

### Normative decisions


| ID                                              | Decision                                                                                                                                                                                                                                                                    | Implementation consequence                                                                                                                                                                                                                                                                |
| ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D3 Ignore directive grammar**                 | Single grammar `specify-ignore: <RULE-ID> — <rationale>` recognised inside known comment delimiters (`//`, `#`, `<!-- -->`, `--`, `/* */`). The directive applies to the next non-blank, non-comment line.                                                                  | New `ignore_directive` fact in `WorkspaceModel`; new extractor in `crates/specify-lints/src/lint/index/`; new `disposition.directive` payload in emitted findings.                                                                                                                                               |
| **D4 Directive-without-rationale is a finding** | An unrationaled directive emits `UNI-022` (`ignore-directive-missing-rationale`); a directive whose `<RULE-ID>` does not match any finding on its target line emits `UNI-023` (`ignore-directive-orphan`).                                                                  | Add two `UNI-`* rules under `adapters/shared/rules/universal/`, but emit the findings from a dedicated directive-validation pass in `crates/specify-lints/src/lint/ignore.rs` rather than through `kind: regex`. `kind: regex` continues to scan file text only; the new pass reads `WorkspaceModel.ignore_directives` and consults the current scan's finding set. |
| **D5 Status enum widened**                      | `LintFinding.status` adds `ignored` to RFC-28's existing `open` / `fixed` / `accepted` / `false-positive` set. [RFC-33b](future/rfc-33b-standards-baseline.md) widens the enum a second time (adding `new` and `baselined`) when it lands. | One additive bump to `schemas/lint/finding.schema.json`; RFC-33b's widening is a second additive bump under the same fingerprint contract. |
| **D6 Disposition object**                       | New optional `disposition: { source, directive?, since? }` on `LintFinding`. RFC-33a producers populate `disposition.directive`. [RFC-33b](future/rfc-33b-standards-baseline.md) adds the optional `disposition.baseline` sub-field when it lands. | One new field in the schema; one new DTO beside `LintFinding` in `crates/specify-lints/src/rules.rs`; populated only when `status != open`. |
| **D8 Journal event**                            | New `lint-completed` variant on the closed `EventKind` taxonomy.                                                                                                                                                                                                          | One new variant in `crates/domain/src/journal.rs`; payload counts per-status; no per-finding detail to keep journal lines bounded. While [RFC-33b](future/rfc-33b-standards-baseline.md) is deferred, `baseline_present` is always `false` and the payload carries `counts.{open, ignored, false_positive}` only; RFC-33b adds `counts.{new, baselined}` and makes `baseline_present` scan-derived when it lands. |
| **D12 Directive rationale stays free-form**     | The grammar is `specify-ignore: <RULE-ID> — <rationale>`. No structured qualifiers (`expires:`, `owner:`, `ticket:`); rationale is opaque text. Rationales shorter than 16 characters emit `UNI-022`. Structured policy lives in rules and baseline metadata, not in source comments.                       | The indexer captures the raw rationale without qualifier parsing; the directive-validation pass performs the length check; downstream tools render rationale as a single string field.                                                                                                                             |
| **D13 First-party rule files**                  | The two new universal rules are filed at fixed paths so the implementation has concrete targets.                                                                                                                                                                      | `adapters/shared/rules/universal/ignore-directive-missing-rationale.md` (id `UNI-022`) and `adapters/shared/rules/universal/ignore-directive-orphan.md` (id `UNI-023`).                                                                                                                   |


The decision IDs (D3, D4, D5, D6, D8, D12, D13) are preserved from the original combined RFC-33 so prior references in commit messages, PRs, and historical discussion continue to resolve. [RFC-33b](future/rfc-33b-standards-baseline.md) owns D1, D2, D7, D9, D10, D11.

### Finding status taxonomy


| Value            | Set by                | Owner                                                       | Meaning                                                                                                                                                            |
| ---------------- | --------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `open`           | scanner (raw)         | RFC-33a                                                     | Default for a freshly emitted finding before post-passes run.                                                                                                      |
| `ignored`        | directive pass        | RFC-33a                                                     | Finding's location matches a `specify-ignore` directive for its `rule-id`. Reported with `disposition.directive` filled in.                                        |
| `fixed`          | diff verb             | RFC-33b (deferred)                                          | Fingerprint present in `last.json`, absent from current scan. Synthetic — emitted only by `specrun lint baseline diff`.                                          |
| `accepted`       | operator              | RFC-33b (deferred)                                          | Set by `specrun lint baseline write --rationale <...>` for entries explicitly accepted.                                                                          |
| `false-positive` | operator or directive | RFC-33a (directive path), RFC-33b (operator path, deferred) | Directive with rationale prefixed `false-positive:` (RFC-33a) or operator baseline entry with `kind: false-positive` (RFC-33b). Reported separately in dashboards. |


`open` is the only RFC-33a value that is CI-blocking by default. `ignored` and `false-positive` are presentation-only. RFC-33b adds `new` as the second CI-blocking value once baseline mode lands.

### Ignore directives

Grammar (one definition, applied across comment styles):

```text
specify-ignore: <RULE-ID> — <rationale>
```

- `<RULE-ID>` is a closed RFC-28 rule id (e.g. `UNI-014`, `OMNIA-021`).
- `—` is an em-dash *or* the two-character sequence `--` (authoring tolerance).
- `<rationale>` is non-empty, free-form text. Rationales shorter than 16 characters are accepted for parsing but emit `UNI-022`.
- One rule per directive. Two adjacent directives on consecutive lines compose: each applies to the same next non-blank, non-comment line.

Comment-style recognition (a closed list, owned by the indexer):


| Language family                                  | Directive syntax accepted                         |
| ------------------------------------------------ | ------------------------------------------------- |
| C-family (Rust, JS, Go, TS, Swift, Java, C, C++) | `// specify-ignore: …`, `/* specify-ignore: … */` |
| Shell, Python, YAML, TOML                        | `# specify-ignore: …`                             |
| HTML, Markdown, XML                              | `<!-- specify-ignore: … -->`                      |
| SQL, Lua                                         | `-- specify-ignore: …`                            |


Scope rules:

- A directive applies to the **next non-blank, non-comment line**. Inline `// specify-ignore: ...` placed at end-of-line on the same line as code applies to that same line.
- File-scoped and block-scoped variants are explicitly **not in v1**. Mass adoption is the baseline file's job (principle 4) and lands with [RFC-33b](future/rfc-33b-standards-baseline.md).
- A directive whose `<RULE-ID>` does not match any finding on its target line emits `UNI-023` (`ignore-directive-orphan`, new shared rule). Operators see dead directives in the same envelope as everything else.

WorkspaceModel addition (one new fact family):

```text
ignore_directive  { path, line, rule_id, rationale?, target_line, raw }
```

The indexer extracts directives once per scan under `crates/specify-lints/src/lint/index/`; it records malformed directives too, with `rationale` absent, so the directive-validation pass can emit `UNI-022`. After deterministic hints emit candidate findings, a dedicated directive pass consumes `WorkspaceModel.ignore_directives`, stamps `status: ignored` plus `disposition.directive` on matches, stamps `status: false-positive` when the rationale begins with `false-positive:`, and emits `UNI-022` / `UNI-023` synthetic findings when the matching validation rules resolved.

### Journal event

Single new variant on the closed `EventKind` taxonomy in `crates/domain/src/journal.rs`:

```text
lint-completed
```

Payload:

```json
{
  "scope": { "target": "omnia", "slice": null, "artifact": null },
  "duration_ms": 824,
  "counts": {
    "open": 12,
    "ignored": 4,
    "false_positive": 0
  },
  "baseline_present": false,
  "exit_code": 2
}
```

While [RFC-33b](future/rfc-33b-standards-baseline.md) is deferred, the scanner always emits `baseline_present: false` and the payload carries only `counts.{open, ignored, false_positive}`. RFC-33b adds `counts.{new, baselined}` to the payload and makes `baseline_present` scan-derived when it lands; both additions are backwards-compatible — consumers tolerant of unknown keys see no regression on the RFC-33a payload.

No per-finding detail in the journal. Operators wanting per-finding history will be able to read `.specify/lint/last.json` once RFC-33b ships, or build their own log on top of `specrun events tail` once RM-14 ships.

### CLI surface

RFC-33a adds no new CLI verbs or flags. The surface remains the existing RM-10 `lint run` subcommand:

```bash
specrun lint run
specrun lint run --slice <name>
specrun lint run --output-format json
```

Directives live in source; the indexer picks them up unconditionally. The journal event fires on every scan completion.

### Exit and presentation semantics

RFC-33a changes the lint exit decision from severity-only to **status-aware severity**: `specrun lint run` exits 2 only when a finding with `status: open` also has `severity: critical` or `severity: important`. Findings with `status: ignored` or `status: false-positive` remain in every formatter and in the JSON envelope, but they do not contribute to the default blocking decision.

The directive-validation findings are ordinary findings. `UNI-022` and `UNI-023` default to `status: open`, so malformed or dead directives block when their severities are `critical` or `important` and stay non-blocking otherwise. Presentation formatters may render status tokens, but RFC-33a does not require a new CLI flag or a new formatter shape.

### Schema changes

Two schema extensions:


| File | Status | Owner |
| --- | --- | --- |
| `schemas/lint/finding.schema.json` | Extended | RFC-28 (this RFC extends the `status` enum and adds the optional `disposition` field) |
| `schemas/lint/workspace-model.schema.json` | Extended | RFC-32 (this RFC adds the `ignore_directives` fact collection) |


Backwards compatibility:

- The fingerprint algorithm does **not** change. RFC-28 already excludes `status` from the fingerprint.
- The enum widening (`ignored`) is additive; consumers that only handle RFC-28's `open` / `fixed` / `accepted` / `false-positive` set see `null` if they reject unknown values, but the schema enum widens cleanly. [RFC-33b](future/rfc-33b-standards-baseline.md) widens the enum a second time (adding `new` and `baselined`) when it lands.
- The new optional `disposition` field is unset when `status: open`, so RFC-28's fingerprint and severity producer contracts are unchanged.

Each enum widening is additive, and consumers using exhaustive matching must already tolerate unknown values under additive schema evolution. Landing only what RFC-33a producers actually emit keeps the schema honest about its real wire shape; RFC-33b widens the enum a second time on a real producer landing rather than pre-reserving values nothing emits.

### Implementation plan

1. **Schema.** Extend `schemas/lint/finding.schema.json` (status enum adds `ignored`; optional `disposition` field added with `source`, `directive?`, `since?` sub-fields) and `schemas/lint/workspace-model.schema.json` (new `ignore_directives` top-level fact collection). Golden test: fingerprint stability across the enum widening, asserted against the RFC-28 Phase 2 fixtures.
2. **Standards-layer types.** Add `FindingDisposition` / `DirectiveDisposition` DTOs beside `LintFinding` in `crates/specify-lints/src/rules.rs`, and add the `IgnoreDirective` DTO plus `WorkspaceModel.ignore_directives` in `crates/specify-lints/src/lint/model.rs`. Reuse RFC-28's canonical-JSON helper for stable serialisation. ([RFC-33b](future/rfc-33b-standards-baseline.md) adds `Baseline`, `BaselineEntry`, `ReviewRun` to the same standards-layer boundary when it lands.)
3. **Indexer fact.** Add the `ignore_directive` extractor under `crates/specify-lints/src/lint/index/`. Honour the closed comment-style list; ignore everything else without falling back to heuristics.
4. **Scanner pipeline.** Insert the directive pass in `crates/specify-lints/src/lint/ignore.rs` between hint evaluation and envelope emission. Order: hint evaluation → default `status: open` assignment → directive validation/matching → ordering → envelope/render → status-aware exit decision. ([RFC-33b](future/rfc-33b-standards-baseline.md) inserts a baseline pass between directive matching and ordering when it lands.)
5. **Journal.** Add the `lint-completed` variant and wire it into the existing emission path. One new event-shape fixture; `baseline_present` hard-coded to `false` in RFC-33a emitters.
6. **First-party rules.** Author the two `UNI-`* rules pinned by D13: `adapters/shared/rules/universal/ignore-directive-missing-rationale.md` (id `UNI-022`) and `adapters/shared/rules/universal/ignore-directive-orphan.md` (id `UNI-023`). They are policy metadata consumed by the directive-validation pass, not `kind: regex` hints; resolver failures degrade per the §"Graceful degradation" rule below.
7. **Acceptance.** Golden tests: scan with directive absent → all `open`; scan with directive present → `ignored` with `disposition.directive`; scan with unrationaled directive → `UNI-022`; scan with orphan directive → `UNI-023`.

### Migration

**For operator projects:** Additive. Without directives, behaviour is identical to RFC-32 Phase 2. Directives may land alongside the code they apply to in the same PR that introduces the protected change.

**For adapter authors:** No required action. Adapter rules may add directives in their own examples; the new shared `UNI-022` / `UNI-023` rules apply to consumer-project code, not to adapter source.

**For framework contributors:** No required action in RFC-32 Phase 2. If RFC-28 Phase 3 (`specdev lint --format json`) is in scope by the time this RFC ships, the same `status` / `disposition` field set applies to framework findings without code changes on the `specdev` side.

**For CLI maintainers:** RFC-33a introduces no filesystem state under `.specify/lint/`. The directory shape is reserved for [RFC-33b](future/rfc-33b-standards-baseline.md).

### Graceful degradation when the universal codex tree is absent

`UNI-022` and `UNI-023` live in `adapters/shared/rules/universal/` and reach the scanner via the RFC-28 codex resolver. On consumer projects that have not yet picked up the shared codex tree (the [RM-10 "Distribution follow-up"](roadmap.md#rm-10-ci-native-standards-enforcement) gap), the resolver simply does not produce those two rules, and the scanner emits neither `UNI-022` nor `UNI-023` findings. Everything else specified by RFC-33a remains functional:

- The `ignore_directive` indexer fact (D3) is extracted from source unconditionally; it does not depend on the codex resolution outcome.
- `disposition.directive` is stamped on findings that match a directive for their `rule-id`, again independent of whether the universal tree resolved.
- The `lint-completed` journal event (D8) operates solely on the current scan envelope; it does not depend on `UNI-022` / `UNI-023` being resolvable.

The only operator-visible regression in this configuration is that unrationaled directives and orphan directives slip through silently. The fix is to either pass `--rules-root` to `specrun lint run` until the distribution follow-up lands, or to vendor the shared tree into the consumer project's `.specify/cache/rules/` location read by `resolve_rules_root` (see [`src/runtime/commands/lint/run.rs`](https://github.com/augentic/specify-cli/blob/main/src/runtime/commands/lint/run.rs)).

This degradation is intentional: it keeps RFC-33a deliverable independently of the RM-10 distribution work, at the cost of a documented caveat for consumer projects that adopt directive tracking before the shared tree is colocated.

## Relationship to RFC-28 / RFC-32 / RFC-33b


| Concern                    | RFC-28  | RFC-32             | RFC-33a                              | RFC-33b (deferred)                        |
| -------------------------- | ------- | ------------------ | ------------------------------------ | ----------------------------------------- |
| `LintFinding` envelope   | Defines | Produces           | Extends `status`; adds `disposition` | (no further change)                       |
| Fingerprint algorithm      | Defines | Computes           | Consumes (join key)                  | Consumes (join key)                       |
| `WorkspaceModel`           | —       | Defines + extracts | Adds `ignore_directive` fact         | (no further change)                       |
| `specrun lint` core scan | —       | Defines            | Adds directive post-pass             | Adds baseline mode + last-run persistence |
| Ignore directive grammar   | —       | —                  | Defines                              | Consumes (for diff `ignored[]` bucket)    |
| Baseline file shape        | —       | —                  | —                                    | Defines                                   |
| Diff between runs          | —       | —                  | —                                    | Defines                                   |
| `EventKind` taxonomy       | —       | —                  | Adds `lint-completed`                | Populates `baseline_present` truthfully   |
| CI blocking semantics      | —       | —                  | `open` blocks by default             | `open` / `new` block by default           |


## Alternatives considered

**File-level or directory-level ignore directives.** Rejected for v1. Encourages "lint shopping" where an inconvenient rule is disabled wholesale rather than baselined with context. The baseline file ([RFC-33b](future/rfc-33b-standards-baseline.md)) solves the mass-adoption case with explicit operator review at write time.

**Use existing `status: accepted` for both baseline and directive matches.** Rejected. Operators need to distinguish "we acknowledged this in the baseline" (RFC-33b, `accepted`) from "we wrote a comment next to the code explaining why" (RFC-33a, `ignored`). Conflating them loses signal in dashboards and PR comments.

**Land RFC-33a and RFC-33b atomically.** Rejected. Under the current "resolve every finding before release" operator policy on a Specify-native codebase, RFC-33b surfaces (baseline file, last-run persistence, diff verb) have no consumers: there is no legacy debt to stage, and the cross-run diff signal collapses because every prior `main`-branch scan is clean. RFC-33a surfaces (directives, telemetry) earn their keep on day one regardless of policy. Landing RFC-33b atomically would invest ~600 LOC of implementation plus the per-target selection matrix and the write/rescan/append decision tree in cognitive burden, all behind opt-in CLI verbs that nobody calls. Pre-designing RFC-33b separately preserves the wire-shape decisions (D5, D6) for RFC-33a to land while deferring the baseline implementation behind concrete trigger conditions in [RFC-33b](future/rfc-33b-standards-baseline.md) §"Trigger conditions".

**Keep RFC-33a and RFC-33b as one combined RFC with a phase split.** Rejected. The combined document covered ~410 lines spanning two distinct operational layers (per-line ignore directives + cross-run baseline). Splitting separates the active surface from the deferred one cleanly, lets each document's references and migration text stay focused, and matches the precedent set by RFC-3a / RFC-3b. The wire-shape decisions (D5, D6) live in RFC-33a and are consumed by RFC-33b without duplication.

## Non-Goals

- PR comment rendering, dashboard aggregation, or any presentation layer beyond CLI output.
- Ignore directives that span ranges, files, or directories.
- Ignoring model-assisted findings before RFC-32 wires LLM producers into the deterministic core. Directives apply to deterministic findings only.
- Workflow lifecycle transitions in any form (RFC-32 principle 7 carries forward).

## Open Questions

RFC-33a has no operational open questions; the directive grammar, indexer fact, status-aware exit decision, and journal payload are fully specified above. Open questions surface as implementation discovers them. The three operational questions previously held against RFC-33b are tracked in [RFC-33b](future/rfc-33b-standards-baseline.md) §"Open Questions".

## References

- [RFC-28: Engineering Standards — Codex Contract and Findings](done/rfc-28-standards-contract.md)
- [RFC-32: Engineering Standards — Deterministic Enforcement](done/rfc-32-standards-enforcement.md)
- [RFC-33b: Standards Baseline](future/rfc-33b-standards-baseline.md) — companion RFC, deferred behind trigger conditions
- [Standards layer (explanation)](../docs/explanation/standards-layer.md)
- [Specify Roadmap — RM-10](roadmap.md#rm-10-ci-native-standards-enforcement)
- [Specify Roadmap — RM-14](roadmap.md#rm-14-local-structured-workflow-events)
- `[crates/domain/src/journal.rs](https://github.com/augentic/specify-cli/blob/main/crates/domain/src/journal.rs)` — closed `EventKind` taxonomy

