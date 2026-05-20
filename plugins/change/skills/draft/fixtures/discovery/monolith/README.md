# Monolith discovery fixture

Pins the `.specify/plans/<name>/discovery.md` shape for a small, purpose-built three-adapter TypeScript monolith passed through [`/change:analyze legacy-code`](../../../../analyze/SKILL.md) via [`plugins/change/skills/draft/briefs/omnia/discovery.md`](../../../briefs/omnia/discovery.md). Sibling of [`mixed-inputs/`](../mixed-inputs/), which pins the combined documentation + legacy-code shape.

This fixture is the acceptance target for:

- [RFC-3a C24](../../../../../../../rfcs/archive/rfc-3a-monoliths.md) — the 1:1 adapter → slice mapping in the propose brief. The three adapters below become three plan entries with `scope.monolith.include` pre-filled from each adapter's `sources:` list. The C24 propose fixture consumes this fixture's `expected/discovery.md` as its starting-state input.

| Path                                                    | Role                                                                                                      |
| ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| [`invocation.txt`](invocation.txt)                      | Operator invocation exercised by this fixture.                                                            |
| [`inputs/`](inputs/)                                    | Three-adapter TypeScript monolith: `src/users/` + `src/auth/` + `src/common/`. Four source files + `package.json`. |
| [`expected/discovery.md`](expected/discovery.md)        | Byte-stable combined output. Three adapter summaries (YAML), alphabetically sorted, all carrying the `<!-- source-key: monolith -->` marker. |
| [`expected/plans/traffic/analyze/monolith/metadata.json`](expected/plans/traffic/analyze/monolith/metadata.json) | Structural-metadata sidecar written by the legacy-code branch of `/change:analyze` alongside `discovery.md`. |
| [`notes.md`](notes.md)                                  | Adapter-level rationale + C24 cross-references + per-adapter clustering signals.                     |

Read [`notes.md`](notes.md) before extending the fixture — adapter boundaries were chosen to exercise specific clustering signals (import edges, docstrings, READMEs) and reordering the `sources:` lists or renaming adapters changes what downstream C24 propose produces.

## Adapters pinned

Three adapters, alphabetical order:

1. **`email-verification`** (`high`) — one file (`src/auth/verify.ts`), two HTTP entry points, `postgres` + `sendgrid` external deps.
2. **`shared-validation`** (`medium`) — one file (`src/common/validation.ts`), no entry points, no external deps; intentionally omits the `hints:` block (a legal shape per the output contract).
3. **`user-registration`** (`high`) — three files spanning `src/users/` and `src/auth/`, depends on `email-verification` and `shared-validation`. **Byte-identical** to the canonical sample entry pinned in [`rfc-3a-monoliths.md` §*Plan-time analysis, define-time extraction*](../../../../../../../rfcs/archive/rfc-3a-monoliths.md) and the Omnia analyze-brief fixture at [`plugins/change/skills/draft/briefs/omnia/fixtures/analyze/legacy-code/expected/discovery.md`](../../../briefs/omnia/fixtures/analyze/legacy-code/expected/discovery.md).

## Relationship to the Omnia analyze fixture

The Omnia fixture at [`plugins/change/skills/draft/briefs/omnia/fixtures/analyze/legacy-code/`](../../../briefs/omnia/fixtures/analyze/legacy-code/) pins the **brief-level** output of `/change:analyze legacy-code` on a four-adapter tree (adds `billing-subscription`). This fixture pins the **plan-level** combined `discovery.md` produced after `/change:draft`'s discovery brief wraps the analyze output in `# Discovery — <name>` + `## Adapter inventory`. Different layers, different scopes, different owners — the two fixtures do not share source trees or expected outputs.

## What this fixture pins

- `# Discovery — traffic` header + `## Adapter inventory` wrapper emitted by [`plugins/change/skills/draft/briefs/omnia/discovery.md`](../../../briefs/omnia/discovery.md) before dispatching to `/change:analyze`.
- Three `### <name>` + fenced YAML blocks, alphabetically sorted by name, all prefixed with `<!-- source-key: monolith -->` (emitted by the skill because `--source monolith=…` supplies the key).
- Fixed YAML field order (`summary`, `sources`, `depends-on`, `hints`, `confidence`) per [`analyze/SKILL.md` §Output contract](../../../../analyze/SKILL.md).
- Alphabetic ordering within `sources`, `depends-on`, `hints.entry_points`, `hints.external_deps`.
- `shared-validation` omits `hints:` entirely (a legal shape).
- **No** `## Constraints (from documentation)` or `## Open questions (from documentation)` blocks — those are documentation-branch-only and this fixture has no documentation inputs.
- `metadata.json` in the C20-pinned v1 shape: six required fields in fixed order, `top_level_modules` alphabetically sorted, no timestamps.

## Not covered here

- Documentation + legacy-code mixed inputs — pinned separately in [`mixed-inputs/`](../mixed-inputs/).
- Out-of-scope / vendored code handling — surfaces at propose time via `scope.<k>.exclude`.
- A `confidence: low` adapter or a tangled-case manifest slice — the Stage C manifest fixture (C27) lands separately.
- A multi-source-key run — this fixture has one source (`monolith`); multi-source discovery is exercised by `mixed-inputs/`.
