# RFC-35: Synthesis Determinism — Closing Reference Contradictions and Agent Trial-and-Error Loops

> Status: Draft · Depends: [RFC-27](done/rfc-27-synthesis.md) (synthesis contract), [RFC-25](done/rfc-25-workflow.md) (workflow), [RFC-31](done/rfc-31-vectis-screenshots-loop.md) (Vectis screenshots loop) · Affects: `/spec:refine` skill body, `specrun slice validate`, `specrun slice fusion`, synthesis references

## Abstract

The `/spec:refine` phase orchestrates artifact synthesis from Evidence through a chain of skill instructions, synthesis references, and target shape briefs. An end-to-end execution of `/spec:execute` against a six-slice Vectis plan exposed six friction points where agents fall into multi-step trial-and-error loops because the guidance is contradictory, ambiguous, or absent. Each friction point has the same root shape: the agent writes an artifact, the validator rejects it, the error message points at the wrong cause, and the agent guesses its way to the correct format through repeated rewrites.

This RFC closes the six friction points with four categories of change:

1. **Reference corrections** — fix three contradictions between `substeps.md`, `spec-format.md`, and the Vectis shape brief so every synthesis reference gives consistent guidance for scenario headings, proposal sections, and spec file paths.
2. **CLI determinism** — add `specrun slice fusion write` and `specrun journal emit` verbs to replace fragile agent-authored YAML and NDJSON with schema-validated CLI output.
3. **Validator diagnostics** — improve `specrun slice validate` error messages to distinguish file-location errors from heading-format errors so agents (and operators) fix the right thing on the first attempt.
4. **Resolver output** — add `briefs_dir` to the `specrun target resolve` and `specrun source resolve` JSON output so agents can locate adapter briefs deterministically.

None of these changes alter the synthesis contract, the slice lifecycle, or the plan-driven loop. They are documentation fixes and CLI conveniences that make the existing contract executable without guesswork.

## Motivation

The friction evidence comes from a single `/spec:execute` session driving six Vectis slices through `refine → build → merge`. The first slice (`app-shell`) consumed the bulk of the session's refine phase due to repeated trial-and-error on artifact format. The six friction points, in order of impact:

### F1. Scenario heading format — `substeps.md` contradicts `spec-format.md`

`substeps.md` line 29 says: "Acceptance scenarios, when needed, live under a `## Scenarios` H2 *after* all requirement blocks." The actual validated format is `#### Scenario:` (H4) inline within each requirement block, as documented in `spec-format.md` line 10 and used throughout the Vectis test brief, `artifact-format.md`, and every worked example in the repository. The agent wrote plain `Scenario:` text, got "requirements have no scenarios", then had to grep through the adapter cache to discover the H4 format. The refine skill's reference list cites `substeps.md`, `requirement-block.md`, `authority.md`, and `claim-fusion.md` — but not `spec-format.md`.

**Cost:** Two failed validation cycles, one grep search, one full spec rewrite.

### F2. `spec.md` file location — generic refine skill vs. Vectis target convention

The refine skill step 4 says "write `proposal.md → spec.md → design.md → tasks.md`" — implying flat files at the slice root. The Vectis shape brief says "One spec file per feature at `specs/<feature>/spec.md`", and `specrun slice create` creates a `specs/` subdirectory. The agent wrote `spec.md` at the slice root. The validator error said "REQ-001 appears in fusion.yaml but no matching `REQ-*` heading exists in spec.md" — which the agent interpreted as a heading-format problem. It spent four steps trying different heading formats before noticing the `specs/` directory and moving the file.

**Cost:** Four failed validation cycles, one directory listing, one file move.

### F3. Proposal section naming — `substeps.md` prescribes defaults Vectis overrides

`substeps.md` says "Required H2 sections, in order: `## Motivation`, `## Scope`, `## Non-goals`." The Vectis shape brief overrides these with `## Source`, `## Why`, `## Crates`, `## Platforms`. The agent followed `substeps.md` first, then had to rewrite after the validator returned `proposal.why-has-content` and `proposal.crates-listed`.

**Cost:** One full proposal rewrite.

### F4. Adapter brief location discovery

The agent needed the Vectis shape brief and source extract briefs. It tried glob patterns in the plugin cache, `find` commands, `specrun target resolve`, `specrun source resolve`, and directory listings — five steps before finding the briefs at `.specify/.cache/manifests/targets/vectis/briefs/shape.md`. The refine skill says "specrun target resolve ... to locate adapters/targets/<target>/briefs/shape.md" but the agent could not derive the filesystem path from the resolve output.

**Cost:** Five discovery steps consuming context and tool calls.

### F5. `fusion.yaml` manual authoring

The refine skill says "No `specrun slice fusion write` verb exists — the skill body is the writer." The agent hand-authored ~100 lines of structured YAML, which then failed validation with drift errors. Since `fusion.yaml` is audit-only and `spec.md` is authoritative (per DECISIONS.md), the index can be mechanically derived.

**Cost:** One failed validation cycle, one full YAML rewrite.

### F6. Journal event manual authoring

The agent composed NDJSON journal events using shell `printf` and `date`, which is fragile (wrong field names, wrong event names, wrong timestamp format, wrong kebab-case wire ids).

**Cost:** Low per-instance, but compounding over six slices times three events each.

### Common pattern

Every friction point follows the same shape: (1) the agent reads guidance, (2) writes an artifact, (3) the validator rejects it, (4) the error message is unhelpful or points at the wrong cause, (5) the agent guesses and retries. The fix in each case is to make the guidance unambiguous and the CLI deterministic so step 2 succeeds on the first attempt.

## Principles

1. **One voice per topic.** When multiple references describe the same artifact format, exactly one is canonical and the others defer to it. Today `substeps.md`, `spec-format.md`, `artifact-format.md`, `requirement-block.md`, and each target's shape brief all describe parts of `spec.md` — sometimes contradictorily.
2. **Errors name the fix.** A validator error message should name the likely cause and the corrective action. "No matching `REQ-*` heading" when the file is in the wrong directory is a misdirection.
3. **Derivable artifacts should be CLI-derived.** When an artifact's content is a deterministic function of other on-disk artifacts, the CLI should own the derivation. Agent-authored YAML that the CLI immediately validates is wasted work.
4. **Resolve outputs should be self-contained.** An agent calling `specrun target resolve` should be able to read the brief's filesystem path from the JSON output without knowing the CLI's internal cache layout.

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Scenario heading in substeps.md** | `substeps.md` line 29 is corrected to document `#### Scenario:` H4 headings inline within each requirement block, matching `spec-format.md` line 10. The `## Scenarios` H2 guidance is removed. | Edit `plugins/spec/references/synthesis/substeps.md`. |
| **D2 Scenario heading in requirement-block.md** | The canonical template in `requirement-block.md` is extended to include `#### Scenario:` with a WHEN/THEN worked example, so an agent reading only `requirement-block.md` gets the full block shape. | Edit `plugins/spec/references/synthesis/requirement-block.md`. |
| **D3 Refine skill references spec-format.md** | The refine skill's References section adds `spec-format.md` alongside the existing synthesis references. | Edit `plugins/spec/skills/refine/SKILL.md`. |
| **D4 Target-specific spec path in substeps.md** | `substeps.md` section 2 notes that the spec file path is target-specific: "The target shape brief determines the spec file path structure (e.g. Vectis uses `specs/<feature>/spec.md`; other targets may use `spec.md` at the slice root)." The refine skill step 4 echoes this note. | Edit `plugins/spec/references/synthesis/substeps.md` and `plugins/spec/skills/refine/SKILL.md`. |
| **D5 Proposal sections are target-governed** | `substeps.md` section 1 changes "Required H2 sections" to "Default H2 sections" and adds: "When the target shape brief specifies different proposal sections, the shape brief takes precedence." | Edit `plugins/spec/references/synthesis/substeps.md`. |
| **D6 `specrun slice fusion write`** | New CLI verb that reads `spec.md` and the slice's `evidence/*.yaml` files, then writes `fusion.yaml` deterministically. The verb owns timestamp generation, the `resolution` enum, claim cross-referencing, and atomic-write discipline. The refine skill step 5 becomes a single CLI invocation. | New handler in `specify-cli` under `crates/domain/src/slice/fusion.rs`; new `specrun slice fusion write $SLICE_NAME --format json` subcommand. |
| **D7 `specrun journal emit`** | New CLI verb that appends one NDJSON event to `.specify/journal.jsonl` with validated event name, generated timestamp, and typed payload. Replaces fragile shell `printf` in skill bodies. | New handler in `specify-cli` under `crates/domain/src/journal/emit.rs`; new `specrun journal emit <event-name> [--payload key=value ...]` subcommand. |
| **D8 Validator file-location diagnostics** | `specrun slice validate` distinguishes "no spec files found at expected path" from "spec file found but heading not matching." When the validator's target-specific expectation (e.g. `specs/<feature>/spec.md`) finds no files, it emits a targeted error naming the expected path pattern rather than reporting missing headings. | Update provenance-parser error paths in `crates/domain/src/validate/`. |
| **D9 `briefs_dir` in resolve output** | `specrun target resolve --format json` and `specrun source resolve --format json` include a `briefs_dir` field giving the absolute filesystem path to the adapter's briefs directory. | Update the JSON serialisation in `crates/domain/src/adapter/`. |

### D1 + D2 — Scenario heading corrections

`substeps.md` section 2 currently ends with:

> Acceptance scenarios, when needed, live under a `## Scenarios` H2 *after* all requirement blocks. Scenarios cite requirements by id (`Given REQ-001 …`) and do not carry their own provenance.

Replace with:

> Each requirement block may include one or more `#### Scenario:` H4 headings after the requirement body and before the next `### Requirement:` heading. Scenarios use WHEN/THEN format (GIVEN is optional context). The `#### Scenario:` heading level is fixed — see `spec-format.md` for the canonical heading conventions. Scenarios do not carry their own provenance lines.

`requirement-block.md` extends the canonical template to show the scenario heading:

```markdown
### Requirement: <Human-readable name>[ <tag>]

ID: REQ-<NNN>
Sources: [<source-key>, <source-key>, …]
Status: <agreed|unknown|conflict|divergence>

<Requirement body — one or more paragraphs.>

#### Scenario: <Scenario name>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>
```

### D4 — Target-specific spec path

`substeps.md` section 2 gains a note after the opening sentence:

> The spec file path is target-specific. The target `shape` brief determines how spec files are organised within the slice directory. For example, Vectis uses `specs/<feature>/spec.md` (one file per `## Crates` entry in `proposal.md`); other targets may use `spec.md` at the slice root. Consult the loaded shape brief before writing spec files.

The refine skill step 4 adds the same note:

> The shape brief determines the spec file organisation; write spec files at the path the shape brief prescribes (e.g. `specs/<crate>/spec.md` for Vectis, `spec.md` for Omnia).

### D6 — `specrun slice fusion write`

```bash
specrun slice fusion write app-shell --format json
```

Behaviour:

- Reads `$SLICE_DIR/specs/*/spec.md` (or `$SLICE_DIR/spec.md`, per the target contract) and parses every `REQ-NNN` block with its `Sources:` lines.
- Reads `$SLICE_DIR/evidence/*.yaml` and indexes claims by `claim-id`.
- Cross-references each requirement's `Sources:` keys against Evidence claims to build the `contributing-claims` list.
- Selects the `resolution` enum value per the existing closed set (`single-source`, `single-value-agreement`, `authority-resolved`, `conflict`).
- Writes `fusion.yaml` atomically (sibling temp file, then rename).
- Emits the `slice.fusion.written` journal event with `{ slice-name, generator, requirement-count }`.
- On `--format json`, prints a summary with `requirement_count` and `resolution_counts`.

Error cases:

- Missing Evidence for a `Sources:` key → `fusion-evidence-missing` (exit 2).
- Duplicate `REQ-NNN` ids → `fusion-duplicate-req` (exit 2).
- No spec files found → `fusion-no-spec-files` (exit 2).

This verb replaces the "step 5" prose in the refine skill with a single CLI call. The downstream `specrun slice validate` drift gate (step 6) remains unchanged — it validates the CLI's output the same way it validated the agent's output.

### D7 — `specrun journal emit`

```bash
specrun journal emit slice.extract.completed \
  --payload slice-name=app-shell \
  --payload source-key=screens
```

Behaviour:

- Validates the event name against the closed `EventKind` taxonomy.
- Generates an ISO-8601 UTC timestamp.
- Serialises the payload as a JSON object with kebab-case keys.
- Appends one NDJSON line to `.specify/journal.jsonl`.
- Exits 0 on success; exit 2 on unknown event name or missing required payload fields.

Required payload fields are enforced per-event-kind (e.g. `slice.extract.completed` requires `slice-name` and `source-key`). The closed taxonomy and required-field map live in `crates/domain/src/journal.rs`, which already owns the `EventKind` enum.

### D8 — Validator diagnostics

When `specrun slice validate` cannot find spec files at the target-expected path, it emits:

```json
{
  "rule": "specs.file-location",
  "message": "No spec files found. Expected specs/<feature>/spec.md (one per Crates entry in proposal.md) but found spec.md at the slice root. Move the file to specs/<feature>/spec.md.",
  "hint": "The target shape brief for vectis requires spec files under specs/."
}
```

When a spec file is found but a `REQ-NNN` from `fusion.yaml` has no matching block, the existing `slice-fusion-drift` error is kept but its message is refined to distinguish the two cases:

- "REQ-001 listed in fusion.yaml but no requirement block with `ID: REQ-001` exists in any spec file under `specs/`."
- vs. "spec.md found at slice root instead of `specs/<feature>/spec.md` — the validator may not be reading the correct file."

### D9 — `briefs_dir` in resolve output

Current `specrun target resolve vectis@v1 --format json` output gains a `briefs_dir` field:

```json
{
  "name": "vectis",
  "version": "v1",
  "path": ".specify/.cache/manifests/targets/vectis",
  "briefs_dir": "/absolute/path/to/.specify/.cache/manifests/targets/vectis/briefs",
  "operations": ["shape", "build", "merge"]
}
```

The same addition applies to `specrun source resolve`. The path is absolute so agents can read briefs without path arithmetic.

## Implementation plan

Six steps, ordered by dependency. Steps 1–3 are documentation-only changes in `augentic/specify`. Steps 4–6 are CLI changes in `augentic/specify-cli`.

1. **Fix `substeps.md` scenario guidance (D1, D5).** Replace the `## Scenarios` H2 paragraph with `#### Scenario:` H4 inline guidance. Add the target-governed proposal-sections caveat. Add the target-specific spec-path note (D4).
2. **Extend `requirement-block.md` with scenario heading (D2).** Add the `#### Scenario:` heading and a WHEN/THEN worked example to the canonical template.
3. **Update refine skill references (D3, D4).** Add `spec-format.md` to the References section. Add the shape-brief-governs-path note to step 4.
4. **Add `specrun slice fusion write` (D6).** New handler under `crates/domain/src/slice/`. Reads spec files and evidence, writes `fusion.yaml` atomically, emits journal event. Golden tests against existing synthesis fixtures.
5. **Add `specrun journal emit` (D7) and improve validator diagnostics (D8).** New handler under `crates/domain/src/journal/`. Update provenance-parser error messages to distinguish file-location from heading-format errors.
6. **Add `briefs_dir` to resolve output (D9).** Update the JSON serialisation for both `specrun target resolve` and `specrun source resolve`.

**Acceptance:** `make check` green on `augentic/specify` after steps 1–3. `cargo make ci` green on `augentic/specify-cli` after steps 4–6. Manual verification: an agent running `/spec:refine` against a Vectis slice completes without trial-and-error on artifact format, file location, or fusion authoring.

## Migration

**For skill authors:** Steps 1–3 are breaking changes to the synthesis references. Any downstream skill or brief that cites `substeps.md`'s `## Scenarios` H2 guidance or `requirement-block.md`'s template without scenarios must be updated in the same PR. A grep for `## Scenarios` across the plugin repo covers this.

**For CLI consumers:** `specrun slice fusion write` is additive — existing agents that hand-author `fusion.yaml` continue to work until they adopt the new verb. `specrun journal emit` is additive — existing `printf`-based journal writes continue to work. The `briefs_dir` field is additive — existing parsers that do not read it are unaffected.

**For target adapter authors:** The `specs.file-location` validator diagnostic (D8) is target-aware — it reads the target manifest to determine the expected spec path structure. Target adapters that expect a non-default spec layout should declare this in their `adapter.yaml` or shape brief. Vectis already does; Omnia and contracts use the default (`spec.md` at slice root).

## Alternatives considered

**Embed the scenario heading in `substeps.md` as a cross-reference to `spec-format.md`.** Rejected. A cross-reference still requires the agent to read a second file, and experience shows agents follow the first concrete example they find. Putting the `#### Scenario:` template directly in both `substeps.md` and `requirement-block.md` eliminates the indirection.

**Make `fusion.yaml` optional.** Rejected. The audit trail it provides (which Evidence claims contributed to which requirements, and how conflicts were resolved) is valuable for operator review. The problem is not the artifact's existence but the authoring burden. A CLI verb that derives it deterministically preserves the audit value at zero agent cost.

**Add a `specrun slice synthesize` verb that runs all four substeps.** Rejected. The synthesis substeps require LLM-driven judgment (grouping claims, writing requirement prose, designing domain models). A single CLI verb cannot own that work. The verb boundary should be at the mechanical steps: `fusion write` and `journal emit`.

**Fold validator improvements into a broader "developer experience" RFC.** Rejected. The diagnostic improvements here are tightly coupled to the synthesis contract and directly motivated by observed agent failures. Separating them from the reference corrections would delay the fix and lose the causal link.

## Non-goals

- Changing the synthesis contract (claim kinds, authority hierarchy, provenance rules). RFC-27 owns those.
- Automating the four synthesis substeps (proposal, spec, design, tasks). Those require LLM judgment.
- Adding new slice lifecycle states or plan-level changes.
- Changing the Evidence schema or the `fusion.yaml` schema. The existing schemas are correct; the problem is who writes them.
- Addressing WASI tool availability or distribution. That is tracked separately.

## References

- [RFC-27: Synthesis](done/rfc-27-synthesis.md) — the synthesis contract this RFC's reference corrections align with.
- [RFC-25: Workflow](done/rfc-25-workflow.md) — the plan-driven loop and slice lifecycle.
- [RFC-31: Vectis Screenshots Loop](done/rfc-31-vectis-screenshots-loop.md) — Vectis target hardening; this RFC addresses synthesis-side friction from the same pipeline.
- [`plugins/spec/references/spec-format.md`](../plugins/spec/references/spec-format.md) — canonical heading conventions for `spec.md`.
- [`plugins/spec/references/synthesis/substeps.md`](../plugins/spec/references/synthesis/substeps.md) — synthesis substep contract corrected by D1, D4, D5.
- [`plugins/spec/references/synthesis/requirement-block.md`](../plugins/spec/references/synthesis/requirement-block.md) — requirement block template extended by D2.
- [`docs/reference/artifact-format.md`](../docs/reference/artifact-format.md) — definitive artifact format reference (already correct on scenario headings).
- [`DECISIONS.md` (specify-cli)](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) — `fusion.yaml` audit-only decision that motivates D6.
