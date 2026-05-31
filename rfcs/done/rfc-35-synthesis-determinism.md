# RFC-35: Synthesis Determinism — Closing Reference Contradictions and Agent Trial-and-Error Loops

> Status: Done · Depends: [RFC-27](rfc-27-synthesis.md) (synthesis contract), [RFC-25](rfc-25-workflow.md) (workflow), [RFC-31](rfc-31-vectis-screenshots-loop.md) (Vectis screenshots loop) · Affects: `/spec:refine` skill body, `specrun slice validate`, `specrun source resolve`, `specrun target resolve`, synthesis references

## Abstract

The `/spec:refine` phase orchestrates artifact synthesis from Evidence through a chain of skill instructions, synthesis references, and target shape briefs. An end-to-end execution of `/spec:execute` against a six-slice Vectis plan exposed six friction points where agents fall into multi-step trial-and-error loops because the guidance is contradictory, ambiguous, or absent. Each friction point has the same root shape: the agent writes an artifact, the validator rejects it, the error message points at the wrong cause, and the agent guesses its way to the correct format through repeated rewrites.

This RFC closes the core retry loops with three categories of change:

1. **Reference corrections** — fix contradictions between `substeps.md`, `spec-format.md`, target shape briefs, and validator expectations so every synthesis reference gives consistent guidance for scenario headings, proposal sections, and spec file paths.
2. **Validator diagnostics** — improve `specrun slice validate` error messages to distinguish file-location errors from heading-format errors so agents (and operators) fix the right thing on the first attempt.
3. **Resolver output** — add `briefs-dir` to the `specrun target resolve` and `specrun source resolve` JSON output so agents can locate adapter briefs deterministically.

None of these changes alter the Evidence schema, `provenance.yaml` schema, slice lifecycle, or plan-driven loop. The RFC deliberately avoids adding new writer or emitter verbs until repeated evidence shows the existing validation gates and clearer authoring guidance are insufficient.

## Motivation

The friction evidence comes from a single `/spec:execute` session driving six Vectis slices through `refine → build → merge`. The first slice (`app-shell`) consumed the bulk of the session's refine phase due to repeated trial-and-error on artifact format. The six friction points, in order of impact:

### F1. Scenario heading format — `substeps.md` contradicts `spec-format.md`

`substeps.md` line 29 says: "Acceptance scenarios, when needed, live under a `## Scenarios` H2 *after* all requirement blocks." The actual validated format is `#### Scenario:` (H4) inline within each requirement block, as documented in `spec-format.md` line 10 and used throughout the Vectis test brief, `artifact-format.md`, and every worked example in the repository. The agent wrote plain `Scenario:` text, got "requirements have no scenarios", then had to grep through the adapter cache to discover the H4 format. The refine skill's reference list cites `substeps.md`, `requirement-block.md`, `authority.md`, and `claim-reconciliation.md` — but not `spec-format.md`.

**Cost:** Two failed validation cycles, one grep search, one full spec rewrite.

### F2. `spec.md` file location — core workflow drift across target briefs

The refine skill step 4 says "write `proposal.md → spec.md → design.md → tasks.md`" — implying flat files at the slice root. The Vectis shape brief correctly says "One spec file per feature at `specs/<feature>/spec.md`", and `specrun slice create` creates a `specs/` subdirectory. The current CLI validator also scans only `specs/**/*.md`; a root-level `spec.md` is invisible to the provenance and adapter-rule passes. However, Omnia and contracts briefs still refer to root `spec.md` in several places, so this is not a Vectis-specific exception — it is a core workflow contradiction. The agent wrote `spec.md` at the slice root. The validator error said "REQ-001 appears in provenance.yaml but no matching `REQ-*` heading exists in spec.md" — which the agent interpreted as a heading-format problem. It spent four steps trying different heading formats before noticing the `specs/` directory and moving the file.

**Cost:** Four failed validation cycles, one directory listing, one file move.

### F3. Proposal section naming — adapter vocabulary leaks into the core workflow

`substeps.md` says "Required H2 sections, in order: `## Motivation`, `## Scope`, `## Non-goals`." The Vectis shape brief says `## Source`, `## Why`, `## Features`, `## Platforms`. The current validator, meanwhile, hard-codes `## Why` and `## Crates` and maps every `## Crates` entry to `specs/<name>/spec.md`. None of these names is a stable core workflow contract, and `Crates` is especially wrong for Vectis features and contracts surfaces. The agent followed `substeps.md` first, then had to rewrite after the validator returned `proposal.why-has-content` and `proposal.crates-listed`.

**Cost:** One full proposal rewrite.

### F4. Adapter brief location discovery

The agent needed the Vectis shape brief and source extract briefs. It tried glob patterns in the plugin cache, `find` commands, `specrun target resolve`, `specrun source resolve`, and directory listings — five steps before finding the briefs at `.specify/.cache/manifests/targets/vectis/briefs/shape.md`. The refine skill says "specrun target resolve ... to locate adapters/targets/<target>/briefs/shape.md" but the agent could not derive the filesystem path from the resolve output.

**Cost:** Five discovery steps consuming context and tool calls.

### F5. `provenance.yaml` manual authoring

The refine skill says "No `specrun slice provenance` verb exists — the skill body is the writer." The agent hand-authored ~100 lines of structured YAML, which then failed validation with drift errors. Since `provenance.yaml` is audit-only and `spec.md` is authoritative (per DECISIONS.md), this is a validation and authoring-guidance problem before it is a new-command problem. The claim-to-requirement mapping itself is not mechanically derivable from `Sources:` lines alone and remains synthesis judgment.

**Cost:** One failed validation cycle, one full YAML rewrite.

### F6. Journal event manual authoring

The agent composed NDJSON journal events using shell `printf` and `date`, which is fragile (wrong field names, wrong event names, wrong timestamp format, wrong kebab-case wire ids). This was lower-impact than the artifact-shape contradictions, and this RFC does not add a general-purpose journal emitter.

**Cost:** Low per-instance, but compounding over six slices times three events each.

### Common pattern

Every high-impact friction point follows the same shape: (1) the agent reads guidance, (2) writes an artifact, (3) the validator rejects it, (4) the error message is unhelpful or points at the wrong cause, (5) the agent guesses and retries. The fix is to make the artifact contract unambiguous and make validation errors name the corrective action, not to add broad CLI surfaces ahead of demonstrated need.

## Principles

1. **One voice per topic.** When multiple references describe the same artifact format, exactly one is canonical and the others defer to it. Today `substeps.md`, `spec-format.md`, `artifact-format.md`, `requirement-block.md`, and each target's shape brief all describe parts of `spec.md` — sometimes contradictorily.
2. **Targets serve the workflow.** Vectis, Omnia, and contracts may interpret a unit differently, but they must not redefine the core artifact layout or proposal handshake. The workflow owns the stable shape; target briefs add idioms inside it.
3. **Errors name the fix.** A validator error message should name the likely cause and the corrective action. "No matching `REQ-*` heading" when the file is in the wrong directory is a misdirection.
4. **Prefer narrower fixes first.** New CLI verbs create long-lived workflow surface area. This RFC adds CLI output and diagnostics only where they directly remove observed ambiguity; richer writer/emitter commands remain future work unless the leaner fixes fail in repeated runs.
5. **Resolve outputs should be self-contained.** An agent calling `specrun target resolve` should be able to read the brief's filesystem path from the JSON output without knowing the CLI's internal cache layout.

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Scenario heading in substeps.md** | `substeps.md` line 29 is corrected to document `#### Scenario:` H4 headings inline within each requirement block, matching `spec-format.md` line 10. The `## Scenarios` H2 guidance is removed. | Edit `plugins/spec/references/synthesis/substeps.md`. |
| **D2 Scenario heading in requirement-block.md** | The canonical template in `requirement-block.md` is extended to include `#### Scenario:` with a WHEN/THEN worked example, so an agent reading only `requirement-block.md` gets the full block shape. | Edit `plugins/spec/references/synthesis/requirement-block.md`. |
| **D3 Refine skill references spec-format.md** | The refine skill's References section adds `spec-format.md` alongside the existing synthesis references. | Edit `plugins/spec/skills/refine/SKILL.md`. |
| **D4 Core spec layout** | The workflow standardises on `specs/<unit>/spec.md` for every target. Vectis maps units to features, Omnia maps units to crates or service surfaces, and contracts maps units to contract surfaces. Root-level `spec.md` is not a valid refine artifact. | Edit `plugins/spec/references/synthesis/substeps.md`, `plugins/spec/skills/refine/SKILL.md`, target shape/build briefs for Vectis/Omnia/contracts, and validator diagnostics. |
| **D5 Core proposal sections** | The workflow standardises the proposal handshake on `## Why`, `## Units`, and `## Non-goals`. Each `## Units` bullet maps one-to-one to `specs/<unit>/spec.md`. Target briefs may add target-specific sections (e.g. Vectis `## Platforms`) but may not rename or replace the core sections. Validator rules change from `proposal.crates-listed` / `cross.proposal-crates-have-specs` to target-neutral `proposal.units-listed` / `cross.proposal-units-have-specs`. | Edit `plugins/spec/references/synthesis/substeps.md`, all target shape briefs, and `specify-cli` validator rules. |
| **D8 Validator file-location diagnostics** | `specrun slice validate` distinguishes "no files under canonical `specs/<unit>/spec.md` layout" from "spec file found but heading not matching." When no files match `specs/**/*.md` but `spec.md` exists at the slice root, it emits a targeted file-location error naming the canonical path pattern. | Update the slice validator and provenance-parser/provenance-drift error paths in `src/runtime/commands/slice/validate.rs` and `crates/domain/src/validate/`. |
| **D9 `briefs-dir` in resolve output** | `specrun target resolve --format json` and `specrun source resolve --format json` include a `briefs-dir` field giving the absolute filesystem path to the adapter's briefs directory. | Update the JSON serialisation in `src/runtime/commands.rs`. |

### D1 + D2 — Scenario heading corrections

`substeps.md` section 2 currently ends with:

> Acceptance scenarios, when needed, live under a `## Scenarios` H2 *after* all requirement blocks. Scenarios cite requirements by id (`Given REQ-001 …`) and do not carry their own provenance.

Replace with:

> Each requirement block may include one or more `#### Scenario:` H4 headings after the requirement body and before the next `### Requirement:` heading. Scenarios use WHEN/THEN format (GIVEN is optional context). The `#### Scenario:` heading level is fixed — see `spec-format.md` for the canonical heading conventions. Scenarios do not carry their own provenance lines.

`requirement-block.md` extends the canonical template to show the scenario heading:

```markdown
### Requirement: <Human-readable name>[ <tag>]

ID: REQ-<NNN>
Sources: [<source>, <source>, …]
Status: <agreed|unknown|conflict|divergence>

<Requirement body — one or more paragraphs.>

#### Scenario: <Scenario name>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>
```

### D4 — Core spec layout

`substeps.md` section 2 replaces every root `spec.md` implication with the canonical layout:

> Specs always live under `specs/<unit>/spec.md`. Each unit is declared in `proposal.md` under `## Units`; each unit name is kebab-case and maps directly to one spec file path. Targets interpret units according to their domain (Vectis feature, Omnia crate/service surface, contracts contract surface), but no target may move the spec file to the slice root or rename the workflow layout.

The refine skill step 4 adds the same note:

> Write one spec file per `proposal.md` `## Units` entry at `specs/<unit>/spec.md`. The target shape brief explains how to choose units for that target, but the file layout is workflow-owned and identical for Vectis, Omnia, and contracts.

Target brief updates:

- Vectis: rename the proposal `## Features` contract to `## Units`; each unit is still a business feature and still produces `specs/<unit>/spec.md`. `## Platforms` remains a Vectis-specific routing section.
- Omnia: replace root `spec.md` references in shape/build/test/crate briefs with the canonical `specs/<unit>/spec.md` layout. For a single generated crate, the unit should normally be the crate name.
- Contracts: replace root `spec.md` references with `specs/<unit>/spec.md`. For a single HTTP API, event family, or schema vocabulary, the unit should be the contract surface slug.

### D5 — Core proposal sections

`substeps.md` section 1 replaces the current `## Motivation` / `## Scope` / `## Non-goals` default with the workflow-owned proposal sections:

```markdown
## Why

<One to three paragraphs explaining why the slice exists.>

## Units

- <unit-slug> — <target-specific meaning and short scope summary>

## Non-goals

- <Out-of-scope behavior or surface, when known>
```

Rules:

- `## Why` is the motivation section the validator checks.
- `## Units` is the only section the validator uses to locate spec files; every unit bullet maps to `specs/<unit>/spec.md`.
- Target briefs may require additional sections after the core sections. For example, Vectis may require `## Platforms`; contracts may require an authorship-mode note. These sections refine build routing but do not replace the core sections.
- Target briefs may describe what a unit means for that target, but they may not rename the `## Units` section or require root-level `spec.md`.

Validator updates:

- `proposal.why-has-content` remains.
- `proposal.crates-listed` becomes `proposal.units-listed` and checks `## Units`.
- `cross.proposal-crates-have-specs` becomes `cross.proposal-units-have-specs` and maps each unit to `specs/<unit>/spec.md`.
- Error details must use target-neutral wording: "unit" / "spec file", not "crate" unless an Omnia-specific build brief is speaking.

### Deferred — provenance and journal writer verbs

This RFC does not add `specrun slice provenance` or `specrun journal emit`. F5 and F6 show real friction, but they do not yet justify new public verbs:

- `provenance.yaml` records synthesis judgment: which Evidence claims contributed to each requirement and how disagreements resolved. The existing schema and `specrun slice validate` drift gate already catch malformed or stale output. The immediate fix is to keep the authoring contract explicit and improve diagnostics where they point at the wrong cause.
- Journal events should be emitted by the deterministic command that owns the state transition or validation pass. A generic skill-facing emitter would move event-shape responsibility into call sites and create a broad API from one low-cost observation.

`provenance.yaml` authoring guidance should stay minimal: one top-level `version`, `slice`, `generated-at`, `generator`, and ordered `requirements[]`; one requirement row per `REQ-*`; each row mirrors `spec.md`'s `Status:` and `Sources:` lines, lists every consulted `(source, id, kind)` under `contributing-claims`, and records one existing `resolution` enum value. The file remains validated by the existing schema and drift checks; this RFC does not change the schema.

If repeated `/spec:execute` runs still show `provenance.yaml` or journal authoring as the dominant source of retries after D1-D5 and D8-D9 land, a later RFC can propose the smallest specific writer surface with fresh evidence.

### D8 — Validator diagnostics

When `specrun slice validate` cannot find spec files under the canonical layout, it emits:

```json
{
  "rule": "specs.file-location",
  "message": "No spec files found. Expected specs/<unit>/spec.md (one per Units entry in proposal.md) but found spec.md at the slice root. Move the file to specs/<unit>/spec.md.",
  "hint": "The Specify workflow requires spec files under specs/ for every target."
}
```

When a spec file is found but a `REQ-NNN` from `provenance.yaml` has no matching block, the existing `slice-provenance-drift` error is kept but its message is refined to distinguish the two cases:

- "REQ-001 listed in provenance.yaml but no requirement block with `ID: REQ-001` exists in any spec file under `specs/`."
- vs. "spec.md found at slice root instead of `specs/<unit>/spec.md` — move the file under `specs/` so the validator can read it."

### D9 — `briefs-dir` in resolve output

Current `specrun target resolve vectis@v1 --format json` output gains a `briefs-dir` field while preserving the existing kebab-case JSON style:

```json
{
  "name": "vectis",
  "axis": "targets",
  "resolved-path": ".specify/.cache/manifests/targets/vectis",
  "location": "cached",
  "briefs-dir": "/absolute/path/to/.specify/.cache/manifests/targets/vectis/briefs",
  "operations": ["shape", "build", "merge"]
}
```

The same addition applies to `specrun source resolve`. The path is absolute so agents can read briefs without path arithmetic.

## Implementation plan

Four steps, ordered by dependency. Steps 1–3 are documentation-only changes in `augentic/specify`. Step 4 is the narrow CLI support needed to make validation and adapter resolution actionable.

1. **Fix `substeps.md` scenario guidance and core artifact contract (D1, D4, D5).** Replace the `## Scenarios` H2 paragraph with `#### Scenario:` H4 inline guidance. Replace target-varying proposal/spec guidance with `## Why`, `## Units`, `## Non-goals` and `specs/<unit>/spec.md`.
2. **Extend `requirement-block.md` with scenario heading (D2).** Add the `#### Scenario:` heading and a WHEN/THEN worked example to the canonical template.
3. **Update refine skill and target briefs (D3, D4, D5).** Add `spec-format.md` to the References section. Update refine step 4 to write `specs/<unit>/spec.md`. Update Vectis, Omnia, and contracts shape/build briefs to use `## Units` and the canonical spec layout.
4. **Improve CLI diagnostics and resolve output (D8, D9).** Update provenance-parser/provenance-drift error messages to distinguish file-location from heading-format errors. Add `briefs-dir` to the JSON serialisation for both `specrun target resolve` and `specrun source resolve`.

**Acceptance:** `make check` green on `augentic/specify` after steps 1–3. `cargo make ci` green on `augentic/specify-cli` after step 4. Manual verification: an agent running `/spec:refine` against Vectis, Omnia, and contracts slices can locate shape briefs from resolver output and writes `proposal.md`, `specs/<unit>/spec.md`, `design.md`, `tasks.md`, and `provenance.yaml` without trial-and-error on artifact format, file location, or proposal vocabulary.

## Migration

**For skill authors:** Steps 1–3 are breaking changes to the synthesis references. Any downstream skill or brief that cites `substeps.md`'s `## Scenarios` H2 guidance, root-level `spec.md`, `## Crates`, or Vectis `## Features` must be updated in the same PR. Grep for `## Scenarios`, `spec.md`, `## Crates`, and `## Features` across target briefs and tests.

**For CLI consumers:** The `briefs-dir` field is additive — existing parsers that do not read it are unaffected. Validator error rules gain more precise wording but do not change lifecycle or artifact schemas.

**For target adapter authors:** The `specs.file-location` validator diagnostic (D8) is workflow-aware, not target-aware: every target uses `specs/<unit>/spec.md`. Target adapters document what a unit means for their domain and may add target-specific proposal sections, but they do not declare alternate spec layouts.

## Alternatives considered

**Embed the scenario heading in `substeps.md` as a cross-reference to `spec-format.md`.** Rejected. A cross-reference still requires the agent to read a second file, and experience shows agents follow the first concrete example they find. Putting the `#### Scenario:` template directly in both `substeps.md` and `requirement-block.md` eliminates the indirection.

**Make `provenance.yaml` optional.** Rejected. The audit trail it provides (which Evidence claims contributed to which requirements, and how conflicts were resolved) is valuable for operator review. The immediate problem is unclear authoring and diagnostics, not the artifact's existence.

**Let each target define its own proposal sections and spec layout.** Rejected. That was the state that produced the Vectis failure, and it would continue to make agents branch on adapter-specific workflow mechanics. Targets should serve the core workflow: they define domain idioms and build routing inside a stable artifact contract.

**Add a `specrun slice provenance` verb now.** Deferred. A writer could remove some YAML envelope risk, but it would introduce a new draft schema and public command surface from one observed `provenance.yaml` failure. The leaner fix is to clarify the existing `provenance.yaml` contract and rely on the current schema plus drift validation.

**Add a `specrun journal emit` verb now.** Rejected for this RFC. Journal events should come from the deterministic command that owns the state change or validation result. A generic emitter is broader than the observed problem and would push event-shape responsibility into every skill call site.

**Add a `specrun slice synthesize` verb that runs all four substeps.** Rejected. The synthesis substeps require LLM-driven judgment (grouping claims, writing requirement prose, designing domain models). A single CLI verb cannot own that work without replacing the core agent synthesis contract.

**Fold validator improvements into a broader "developer experience" RFC.** Rejected. The diagnostic improvements here are tightly coupled to the synthesis contract and directly motivated by observed agent failures. Separating them from the reference corrections would delay the fix and lose the causal link.

## Non-goals

- Changing the synthesis contract (claim kinds, authority hierarchy, provenance rules). RFC-27 owns those.
- Automating the four synthesis substeps (proposal, spec, design, tasks). Those require LLM judgment.
- Adding new slice lifecycle states or plan-level changes.
- Changing the Evidence schema or the `provenance.yaml` schema. The existing schemas are correct; the problem is contradictory guidance and misleading diagnostics.
- Adding new `provenance.yaml` writer or generic journal-emitter verbs.
- Addressing WASI tool availability or distribution. That is tracked separately.

## References

- [RFC-27: Synthesis](rfc-27-synthesis.md) — the synthesis contract this RFC's reference corrections align with.
- [RFC-25: Workflow](rfc-25-workflow.md) — the plan-driven loop and slice lifecycle.
- [RFC-31: Vectis Screenshots Loop](rfc-31-vectis-screenshots-loop.md) — Vectis target hardening; this RFC addresses synthesis-side friction from the same pipeline.
- [`plugins/spec/references/spec-format.md`](../../plugins/spec/references/spec-format.md) — canonical heading conventions for `spec.md`.
- [`plugins/spec/references/synthesis/substeps.md`](../../plugins/spec/references/synthesis/substeps.md) — synthesis substep contract corrected by D1, D4, D5.
- [`plugins/spec/references/synthesis/requirement-block.md`](../../plugins/spec/references/synthesis/requirement-block.md) — requirement block template extended by D2.
- [`docs/reference/artifact-format.md`](../../docs/reference/artifact-format.md) — definitive artifact format reference (already correct on scenario headings).
- [`DECISIONS.md` (specify-cli)](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) — `provenance.yaml` audit-only decision that keeps this RFC focused on guidance and validation rather than new authoring verbs.
