---
id: discovery
description: Read --from artefacts and/or analyse codebases; emit a neutral capability inventory.
generates: .specify/plans/<name>/discovery.md
---

Produce a neutral, schema-agnostic capability inventory for the change. Discovery is read-only: it does NOT write to `plan.yaml` and does NOT propose slices. Its only output is the inventory that `propose.md` will decompose.

Every input reaching this brief carries a `kind` drawn from the closed enum `{legacy-code, documentation}`. Any other value is a brief-level error; the brief must refuse to produce an inventory rather than guess a default or silently skip the input. Kind assignment for CLI-supplied inputs is performed upstream by `/change:plan` (see its SKILL.md §*Kind defaults for CLI flags*); this brief receives pre-classified inputs and does not re-apply defaults.

## Inputs

- `--from <path>...` — artefact files or directories authored by a human (briefs, RFCs, product docs, ADRs). Zero or more. Kind is `documentation` unless an explicit suffix overrode it upstream.
- `--against <path>` — an existing codebase to delta against. At most one. Interpreted as a local filesystem path. Kind is `legacy-code` unless an explicit suffix overrode it upstream.
- `--source <key>=<path-or-url>...` — named sources for migration or legacy analysis. `<path-or-url>` is either a local path or a git URL. Zero or more. The `<key>` is the identifier recorded on each plan entry's `sources` list in the next brief. Kind is `legacy-code` unless an explicit suffix overrode it upstream.

At least one of `--from`, `--against`, or `--source` must be supplied.

## Input dispatch (per-kind)

Every input reaching this brief is pre-classified by kind (see [`../../SKILL.md` §*Kind defaults for CLI flags*](../../SKILL.md)). Dispatch per kind — one [`/spec:analyze`](../../../../../spec/skills/analyze/SKILL.md) invocation per input, processed in CLI declaration order (`from` entries before `source` entries before `against`; within each flag, left-to-right), and `change.md:inputs[]` entries interleaved in file order after the CLI inputs of matching kind.

### `kind: documentation`

For each documentation input, invoke [`/spec:analyze`](../../../../../spec/skills/analyze/SKILL.md):

```text
/spec:analyze <input-path> <plan-dir> documentation <k>
```

where `<plan-dir>` is `.specify/plans/<change-name>/` — the same directory this brief writes `discovery.md` to — and `<k>` is the source key used to tag the emitted capabilities:

- `--from <p>` → `--source-key <basename(p) without extension, kebab-cased>`.
- `--source <k>=<p>:documentation` → `--source-key <k>`.
- `--against <p>:documentation` → `--source-key against`.
- `change.md:inputs[]` with `kind: documentation` → `--source-key <basename(path) without extension, kebab-cased>` (v1 brief schema has no `key:` field on `inputs[]`; adding one requires an RFC update per the closed-enum posture).

`/spec:analyze` appends capability summaries to `<plan-dir>/discovery.md` in the shape pinned at [`analyze/SKILL.md` §Output contract](../../../../../spec/skills/analyze/SKILL.md), plus the documentation-branch `## Constraints (from documentation)` and `## Open questions (from documentation)` appendix blocks pinned in [`analyze.md` §Documentation branch](./analyze.md).

### `kind: legacy-code`

For each legacy-code input, invoke [`/spec:analyze`](../../../../../spec/skills/analyze/SKILL.md):

```text
/spec:analyze <input-path> <plan-dir> legacy-code <k>
```

where `<plan-dir>` is `.specify/plans/<change-name>/` and `<k>` is the source key used to tag the emitted capabilities:

- `--source <k>=<p>` (default kind `legacy-code`) → `--source-key <k>`.
- `--source <k>=<p>:legacy-code` (explicit) → `--source-key <k>`.
- `--against <p>` (default kind `legacy-code`) → `--source-key against`.
- `change.md:inputs[]` with `kind: legacy-code` → `--source-key <basename(path) kebab-cased>`.

For a git-URL `--source`, materialise the URL into `legacy/<key>/` with the inlined guarded `git clone` snippet (see [`../../../../../spec/skills/analyze/SKILL.md` §*Cloning a source tree*](../../../../../spec/skills/analyze/SKILL.md)) and pass that local path to `/spec:analyze`.

`/spec:analyze` appends capability summaries to `<plan-dir>/discovery.md` and writes structural metadata to `<plan-dir>/analyze/<k>/metadata.json` (see [`analyze/SKILL.md` §Structural metadata](../../../../../spec/skills/analyze/SKILL.md) and [`analyze.md` §Legacy-code branch](./analyze.md)). This brief does NOT post-process either artifact.

## Merge rule

`/spec:analyze` owns append semantics for capability summaries: dedup-by-name, alphabetic sort, byte-stable output. This brief invokes analyze once per input in CLI declaration order; the final `discovery.md` inherits analyze's idempotency contract. Both documentation and legacy-code inputs share the single capability- summary shape defined in [`analyze/SKILL.md` §Output contract](../../../../../spec/skills/analyze/SKILL.md).

Documentation-kind inputs additionally contribute `## Constraints (from documentation)` and `## Open questions (from documentation)` appendix blocks at the end of `discovery.md` (see [`analyze.md` §Documentation branch](./analyze.md) for their shape). Legacy-code inputs emit only capability summaries.

This brief does NOT re-write, re-sort, or re-deduplicate analyze's output; analyze owns the append contract.

## Output

`discovery.md` has this shape:

````markdown
# Discovery — <change-name>

## Capability inventory

<!-- source-key: <k> -->
### <capability-name>

```yaml
summary: <one-sentence imperative description>
sources:
  - <literal artefact path, optionally with fragment>
depends-on: [<other capability names>]
hints:
  entry_points: [<trigger / command / HTTP verb-path strings>]
  external_deps: [<named external systems>]
confidence: <high | medium | low>
```

<!-- repeat one block per capability, alphabetically sorted by name -->

## Constraints (from documentation)

- <constraint text> (source: <artifact path[#fragment]>)

## Open questions (from documentation)

- <question text> (source: <artifact path[#fragment]>)
````

Section rules:

- The `# Discovery — <change-name>` header and the `## Capability inventory` wrapper are written by this brief (the first thing discovery writes before invoking `/spec:analyze`). `/spec:analyze` appends its `### <name>` blocks — each preceded by a `<!-- source-key: <k> -->` marker — under the wrapper.
- Every capability block is emitted by `/spec:analyze` regardless of kind; both branches share the single YAML shape pinned in [`analyze/SKILL.md` §Output contract](../../../../../spec/skills/analyze/SKILL.md).
- The `## Constraints (from documentation)` and `## Open questions (from documentation)` blocks are documentation-branch-only. Omit either heading when empty; never emit an empty section. If no documentation inputs were supplied, both headings are absent.
- A run with only legacy-code inputs produces `## Capability inventory` followed by nothing else. A run with only documentation inputs produces `## Capability inventory` followed by the two appendix blocks (when non-empty).

## Idempotency

Running discovery twice on the same inputs MUST produce the same `discovery.md`. The contract is inherited from `/spec:analyze`:

- Capabilities alphabetically sorted by `name`.
- Fixed field order inside each YAML block (`summary`, `sources`, `depends-on`, `hints`, `confidence`).
- `sources`, `depends-on`, `hints.entry_points`, and `hints.external_deps` sorted alphabetically.
- No timestamps, run IDs, working-directory paths, or absolute paths anywhere in the file.
- Re-runs on unchanged sources yield byte-equivalent output; any new detail replaces the prior entry wholesale rather than appending a parallel record.

See [`analyze/SKILL.md` §Idempotency](../../../../../spec/skills/analyze/SKILL.md) for the authoritative contract.

## Example fragment

See [`../../fixtures/discovery/monolith/expected/discovery.md`](../../fixtures/discovery/monolith/expected/discovery.md) for a single-`--source` legacy-code invocation that produces three capability summaries (no documentation appendix), and [`../../fixtures/discovery/mixed-inputs/expected/discovery.md`](../../fixtures/discovery/mixed-inputs/expected/discovery.md) for a mixed documentation + legacy-code invocation that produces four capability summaries plus the documentation appendix blocks.
