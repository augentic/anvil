# Monolith discovery fixture

Pins the `.specify/plans/<name>/discovery.md` shape for a small, purpose-built three-capability TypeScript monolith passed through [`/spec:analyze --kind legacy-code`](../../../../../../spec/skills/analyze/SKILL.md) via [`plugins/change/skills/plan/briefs/omnia/discovery.md`](../../../briefs/omnia/discovery.md). Sibling of [`mixed-inputs/`](../mixed-inputs/), which pins the combined documentation + legacy-code shape.

This fixture is the acceptance target for:

- [RFC-3a C24](../../../../../../../rfcs/archive/rfc-3a-monoliths.md) — the 1:1 capability → slice mapping in the propose brief. The three capabilities below become three plan entries with `scope.monolith.include` pre-filled from each capability's `sources:` list. The C24 propose fixture consumes this fixture's `expected/discovery.md` as its starting-state input.

| Path                                                    | Role                                                                                                      |
| ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| [`invocation.txt`](invocation.txt)                      | Operator invocation exercised by this fixture.                                                            |
| [`inputs/`](inputs/)                                    | Three-capability TypeScript monolith: `src/users/` + `src/auth/` + `src/common/`. Four source files + `package.json`. |
| [`expected/discovery.md`](expected/discovery.md)        | Byte-stable combined output. Three capability summaries (YAML), alphabetically sorted, all carrying the `<!-- source-key: monolith -->` marker. |
| [`expected/plans/traffic/analyze/monolith/metadata.json`](expected/plans/traffic/analyze/monolith/metadata.json) | Structural-metadata sidecar written by the legacy-code branch of `/spec:analyze` alongside `discovery.md`. |
| [`notes.md`](notes.md)                                  | Capability-level rationale + C24 cross-references + per-capability clustering signals.                     |

Read [`notes.md`](notes.md) before extending the fixture — capability boundaries were chosen to exercise specific clustering signals (import edges, docstrings, READMEs) and reordering the `sources:` lists or renaming capabilities changes what downstream C24 propose produces.

## Capabilities pinned

Three capabilities, alphabetical order:

1. **`email-verification`** (`high`) — one file (`src/auth/verify.ts`), two HTTP entry points, `postgres` + `sendgrid` external deps.
2. **`shared-validation`** (`medium`) — one file (`src/common/validation.ts`), no entry points, no external deps; intentionally omits the `hints:` block (a legal shape per the output contract).
3. **`user-registration`** (`high`) — three files spanning `src/users/` and `src/auth/`, depends on `email-verification` and `shared-validation`. **Byte-identical** to the canonical sample entry pinned in [`rfc-3a-monoliths.md` §*Plan-time analysis, define-time extraction*](../../../../../../../rfcs/archive/rfc-3a-monoliths.md) and the Omnia analyze-brief fixture at [`plugins/change/skills/plan/briefs/omnia/fixtures/analyze/legacy-code/expected/discovery.md`](../../../briefs/omnia/fixtures/analyze/legacy-code/expected/discovery.md).

## Relationship to the Omnia analyze fixture

The Omnia fixture at [`plugins/change/skills/plan/briefs/omnia/fixtures/analyze/legacy-code/`](../../../briefs/omnia/fixtures/analyze/legacy-code/) pins the **brief-level** output of `/spec:analyze --kind legacy-code` on a four-capability tree (adds `billing-subscription`). This fixture pins the **plan-level** combined `discovery.md` produced after `/change:plan`'s discovery brief wraps the analyze output in `# Discovery — <name>` + `## Capability inventory`. Different layers, different scopes, different owners — the two fixtures do not share source trees or expected outputs.

## What this fixture pins

- `# Discovery — traffic` header + `## Capability inventory` wrapper emitted by [`plugins/change/skills/plan/briefs/omnia/discovery.md`](../../../briefs/omnia/discovery.md) before dispatching to `/spec:analyze`.
- Three `### <name>` + fenced YAML blocks, alphabetically sorted by name, all prefixed with `<!-- source-key: monolith -->` (emitted by the skill because `--source monolith=…` supplies the key).
- Fixed YAML field order (`summary`, `sources`, `depends-on`, `hints`, `confidence`) per [`analyze/SKILL.md` §Output contract](../../../../../../spec/skills/analyze/SKILL.md).
- Alphabetic ordering within `sources`, `depends-on`, `hints.entry_points`, `hints.external_deps`.
- `shared-validation` omits `hints:` entirely (a legal shape).
- **No** `## Constraints (from documentation)` or `## Open questions (from documentation)` blocks — those are documentation-branch-only and this fixture has no documentation inputs.
- `metadata.json` in the C20-pinned v1 shape: six required fields in fixed order, `top_level_modules` alphabetically sorted, no timestamps.

## Not covered here

- Documentation + legacy-code mixed inputs — pinned separately in [`mixed-inputs/`](../mixed-inputs/).
- Out-of-scope / vendored code handling — surfaces at propose time via `scope.<k>.exclude`.
- A `confidence: low` capability or a tangled-case manifest slice — the Stage C manifest fixture (C27) lands separately.
- A multi-source-key run — this fixture has one source (`monolith`); multi-source discovery is exercised by `mixed-inputs/`.
