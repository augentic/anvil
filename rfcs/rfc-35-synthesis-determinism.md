# RFC-35: Synthesis Determinism

> Status: Draft · Depends: [RFC-27](done/rfc-27-synthesis.md), [RFC-25](done/rfc-25-workflow.md), [RFC-31](done/rfc-31-vectis-screenshots-loop.md) · Affects: `/spec:refine` skill, `specrun slice validate`, `specrun slice reconciliation`, synthesis references

## Abstract

A six-slice Vectis `/spec:execute` run exposed six friction points where `/spec:refine` agents fall into trial-and-error loops: the agent writes an artifact, the validator rejects it, the error points at the wrong cause, and the agent guesses through repeated rewrites. This RFC closes them with documentation fixes and CLI conveniences — no change to the synthesis contract, slice lifecycle, or plan-driven loop.

## Motivation

Each friction point observed in the run, with its fix mapped to a decision below:

| # | Friction | Root cause | Fix |
| --- | --- | --- | --- |
| F1 | Wrote plain `Scenario:` text; validator said "requirements have no scenarios" | `substeps.md` says scenarios live under a `## Scenarios` H2; the validated format is `#### Scenario:` (H4) inline per requirement block, per `spec-format.md`. Refine skill doesn't cite `spec-format.md`. | D1, D2, D3 |
| F2 | Wrote `spec.md` at slice root; validator reported missing `REQ-*` heading | Refine skill implies flat files; Vectis shape brief requires `specs/<feature>/spec.md`. Error misdirected to heading format. | D4, D8 |
| F3 | Wrote default proposal sections, then rewrote | `substeps.md` prescribes `## Motivation`/`## Scope`/`## Non-goals`; Vectis overrides with `## Source`/`## Why`/`## Crates`/`## Platforms`. | D5 |
| F4 | Five steps to locate adapter briefs | `specrun target resolve` output gives no filesystem path to the briefs directory. | D9 |
| F5 | Hand-authored ~100 lines of `reconciliation.yaml`; failed drift validation | `reconciliation.yaml` is audit-only and derivable from `spec.md` + evidence, but no CLI verb writes it. | D6 |
| F6 | Composed journal NDJSON with shell `printf`/`date` | Fragile field names, event names, and timestamp format. | D7 |

## Principles

1. **One voice per topic.** When multiple references describe the same artifact format, one is canonical and the others defer to it.
2. **Errors name the fix.** A validator error should name the likely cause and corrective action, not a downstream symptom.
3. **Derivable artifacts are CLI-derived.** When content is a deterministic function of other on-disk artifacts, the CLI owns the derivation.
4. **Resolve outputs are self-contained.** An agent should read a brief's filesystem path from resolve JSON without knowing the cache layout.

## Design

| ID | Decision | Implementation |
| --- | --- | --- |
| **D1** | Correct `substeps.md` to document `#### Scenario:` H4 headings inline per requirement block; remove the `## Scenarios` H2 guidance. | Edit `plugins/spec/references/synthesis/substeps.md`. |
| **D2** | Extend the `requirement-block.md` canonical template to include `#### Scenario:` with a WHEN/THEN example. | Edit `plugins/spec/references/synthesis/requirement-block.md`. |
| **D3** | Add `spec-format.md` to the refine skill's References section. | Edit `plugins/spec/skills/refine/SKILL.md`. |
| **D4** | Note in `substeps.md` and refine skill step 4 that the spec file path is target-specific (Vectis `specs/<feature>/spec.md`; others may use `spec.md` at slice root) and the shape brief governs. | Edit `substeps.md` and `refine/SKILL.md`. |
| **D5** | Change `substeps.md` "Required H2 sections" to "Default H2 sections" with: "When the target shape brief specifies different proposal sections, the shape brief takes precedence." | Edit `substeps.md`. |
| **D6** | New `specrun slice reconciliation write` verb that derives `reconciliation.yaml` from `spec.md` + `evidence/*.yaml`. | New handler `crates/workflow/src/slice/reconciliation.rs`. |
| **D7** | New `specrun journal emit` verb that appends one validated NDJSON event to `.specify/journal.jsonl`. | New handler `crates/workflow/src/journal/emit.rs`. |
| **D8** | `specrun slice validate` distinguishes "no spec files at expected path" from "spec file found but heading not matching." | Update provenance-parser error paths in `crates/validate/src/`. |
| **D9** | Add absolute `briefs_dir` to `specrun target resolve` and `specrun source resolve` JSON output. | Update JSON serialisation in `crates/workflow/src/adapter/`. |

### D1 + D2 — Scenario headings

Replace the `## Scenarios` H2 paragraph in `substeps.md` section 2 with:

> Each requirement block may include one or more `#### Scenario:` H4 headings after the requirement body and before the next `### Requirement:` heading. Scenarios use WHEN/THEN format (GIVEN is optional context); the heading level is fixed (see `spec-format.md`). Scenarios do not carry their own provenance lines.

Extend the `requirement-block.md` template to:

```markdown
### Requirement: <Human-readable name>[ <tag>]

ID: REQ-<NNN>
Sources: [<source-key>, …]
Status: <agreed|unknown|conflict|divergence>

<Requirement body.>

#### Scenario: <Scenario name>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>
```

### D4 — Target-specific spec path

`substeps.md` section 2 and refine skill step 4 gain a note: the target `shape` brief determines spec file organisation. Vectis uses `specs/<feature>/spec.md` (one per `## Crates` entry in `proposal.md`); other targets may use `spec.md` at the slice root. Consult the loaded shape brief before writing spec files.

### D6 — `specrun slice reconciliation write`

```bash
specrun slice reconciliation write app-shell --format json
```

- Parses every `REQ-NNN` block (with `Sources:` lines) from `$SLICE_DIR/specs/*/spec.md` (or `$SLICE_DIR/spec.md`, per target).
- Indexes claims from `$SLICE_DIR/evidence/*.yaml` by `claim-id`; cross-references each requirement's `Sources:` keys to build `contributing-claims`.
- Selects the `resolution` enum from the closed set (`single-source`, `single-value-agreement`, `authority-resolved`, `conflict`).
- Writes `reconciliation.yaml` atomically; emits `slice.reconciliation.written` with `{ slice-name, generator, requirement-count }`; on `--format json` prints `requirement_count` + `resolution_counts`.

Errors (exit 2): `reconciliation-evidence-missing`, `reconciliation-duplicate-req`, `reconciliation-no-spec-files`. The downstream `specrun slice validate` drift gate is unchanged.

### D7 — `specrun journal emit`

```bash
specrun journal emit slice.extract.completed \
  --payload slice-name=app-shell --payload source-key=screens
```

- Validates the event name against the closed `EventKind` taxonomy; generates an ISO-8601 UTC timestamp; serialises payload as a JSON object with kebab-case keys; appends one NDJSON line.
- Exit 2 on unknown event name or missing required payload fields. Required fields are enforced per event kind; the taxonomy and required-field map live in `crates/workflow/src/journal.rs`.

### D8 — Validator diagnostics

When no spec files exist at the target-expected path:

```json
{
  "rule": "specs.file-location",
  "message": "No spec files found. Expected specs/<feature>/spec.md but found spec.md at the slice root. Move the file to specs/<feature>/spec.md.",
  "hint": "The target shape brief for vectis requires spec files under specs/."
}
```

The existing `slice-reconciliation-drift` error is kept but reworded to distinguish a genuinely missing `REQ-NNN` block from a spec file in the wrong directory.

### D9 — `briefs_dir` in resolve output

`specrun target resolve vectis@v1 --format json` (and `source resolve`) gain an absolute `briefs_dir`:

```json
{
  "name": "vectis",
  "version": "v1",
  "path": ".specify/.cache/manifests/targets/vectis",
  "briefs_dir": "/absolute/path/to/.specify/.cache/manifests/targets/vectis/briefs",
  "operations": ["shape", "build", "merge"]
}
```

## Implementation plan

Steps 1–3 are documentation-only in `augentic/specify`; 4–6 are CLI changes in `augentic/specify-cli`.

1. Fix `substeps.md`: `#### Scenario:` H4 guidance (D1), target-governed proposal sections (D5), target-specific spec-path note (D4).
2. Extend `requirement-block.md` with the scenario heading + WHEN/THEN example (D2).
3. Update refine skill: add `spec-format.md` reference (D3); add shape-brief-governs-path note to step 4 (D4).
4. Add `specrun slice reconciliation write` (D6); golden tests against existing synthesis fixtures.
5. Add `specrun journal emit` (D7) and the validator file-location diagnostics (D8).
6. Add `briefs_dir` to both resolve outputs (D9).

**Acceptance:** `make lint` green after 1–3; `cargo make ci` green after 4–6. An agent running `/spec:refine` against a Vectis slice completes without trial-and-error on format, file location, or reconciliation authoring.

## Migration

- **Skill authors:** Steps 1–3 break references citing the `## Scenarios` H2 guidance or the scenario-less `requirement-block.md` template; update them in the same PR (grep `## Scenarios`).
- **CLI consumers:** `reconciliation write`, `journal emit`, and `briefs_dir` are all additive; existing hand-authoring and `printf` writes continue to work until adopted.
- **Target authors:** The `specs.file-location` diagnostic (D8) reads the target manifest for the expected spec layout. Vectis already declares `specs/`; Omnia and contracts use the default `spec.md` at slice root.

## Alternatives considered

- **Cross-reference `spec-format.md` from `substeps.md` instead of inlining the template.** Rejected — agents follow the first concrete example they find; inlining eliminates the indirection.
- **Make `reconciliation.yaml` optional.** Rejected — its audit trail is valuable; the problem is the authoring burden, which D6 removes.
- **A single `specrun slice synthesize` verb for all four substeps.** Rejected — proposal/spec/design/tasks require LLM judgment. The verb boundary stays at the mechanical steps.

## Non-goals

- The synthesis contract (claim kinds, authority, provenance) — owned by RFC-27.
- Automating the four LLM-driven synthesis substeps.
- New slice lifecycle states or plan-level changes.
- Changes to the Evidence or `reconciliation.yaml` schemas — the problem is who writes them, not their shape.

## References

- [RFC-27: Synthesis](done/rfc-27-synthesis.md) — the contract these corrections align with.
- [RFC-29: Fan-In/Fan-Out](rfc-29-fan-in-fan-out.md) — sequenced after this RFC; reuses D6/D7/D9 (see RFC-29 §"Relationship to RFC-35").
- [`spec-format.md`](../plugins/spec/references/spec-format.md), [`substeps.md`](../plugins/spec/references/synthesis/substeps.md), [`requirement-block.md`](../plugins/spec/references/synthesis/requirement-block.md), [`artifact-format.md`](../docs/reference/artifact-format.md).
- [`DECISIONS.md` (specify-cli)](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) — `reconciliation.yaml` audit-only decision motivating D6.
