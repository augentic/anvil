---
id: discovery
description: Read --from artefacts and/or analyse codebases; emit a neutral adapter inventory.
generates: .specify/plans/<name>/discovery.md
---

Produce a neutral, schema-agnostic adapter inventory for the change. Discovery is read-only: it does NOT write to `plan.yaml` and does NOT propose slices. Its only output is the inventory that `propose.md` will decompose.

Every input reaching this brief carries a `kind` drawn from the closed enum `{legacy-code, documentation}`. Any other value is a brief-level error; the brief must refuse to produce an inventory rather than guess a default or silently skip the input. Kind assignment for CLI-supplied inputs is performed upstream by `/change:draft` (see its SKILL.md §*Kind defaults for CLI flags*); this brief receives pre-classified inputs and does not re-apply defaults.

## Inputs

- `--from <path>...` — artefact files or directories authored by a human (briefs, RFCs, product docs, ADRs). Zero or more. Kind is `documentation` unless an explicit suffix overrode it upstream.
- `--against <path>` — an existing codebase to delta against. At most one. Interpreted as a local filesystem path. Kind is `legacy-code` unless an explicit suffix overrode it upstream.
- `--source <key>=<path-or-url>...` — named sources for migration or legacy analysis. `<path-or-url>` is either a local path or a git URL. Zero or more. The `<key>` is the identifier recorded on each plan entry's `sources` list in the next brief. Kind is `legacy-code` unless an explicit suffix overrode it upstream.

At least one of `--from`, `--against`, or `--source` must be supplied.

## Candidate inventory heading

Before dispatching any input to `/change:analyze` or `/change:survey`, write the `## Candidate inventory` heading into `discovery.md` exactly once. Both downstream skills append candidate blocks under this heading; neither writes it. The heading is idempotent — check before writing:

```bash
grep -q '^## Candidate inventory' "$PLAN_DIR/discovery.md" || printf '\n## Candidate inventory\n' >> "$PLAN_DIR/discovery.md"
```

This heading is the handshake contract: `/change:analyze` (for `documentation` inputs) and `/change:survey` (for `legacy-code` inputs) both assume it exists and append under it.

## Input dispatch (per-kind)

Every input reaching this brief is pre-classified by kind (see [`../../SKILL.md` §*Kind defaults for CLI flags*](../../SKILL.md)). Dispatch per kind — processed in CLI declaration order (`from` entries before `source` entries before `against`; within each flag, left-to-right), and `change.md:inputs[]` entries interleaved in file order after the CLI inputs of matching kind.

### `kind: documentation`

For each documentation input, invoke [`/change:analyze`](../../../analyze/SKILL.md):

```text
/change:analyze <input-path> <plan-dir> documentation <k>
```

where `<plan-dir>` is `.specify/plans/<change-name>/` — the same directory this brief writes `discovery.md` to — and `<k>` is the source key used to tag the emitted adapters:

- `--from <p>` → `--source-key <basename(p) without extension, kebab-cased>`.
- `--source <k>=<p>:documentation` → `--source-key <k>`.
- `--against <p>:documentation` → `--source-key against`.
- `change.md:inputs[]` with `kind: documentation` → `--source-key <basename(path) without extension, kebab-cased>` (v1 brief schema has no `key:` field on `inputs[]`; adding one requires an RFC update per the closed-enum posture).

`/change:analyze` appends candidate blocks to `<plan-dir>/discovery.md` under the `## Candidate inventory` heading in the shape pinned at [`analyze/SKILL.md` §Output contract](../../../analyze/SKILL.md), plus the documentation-branch `## Constraints (from documentation)` and `## Open questions (from documentation)` appendix blocks pinned in [`analyze.md` §Documentation branch](./analyze.md).

### `kind: legacy-code`

Legacy-code inputs are handled by [`/change:survey`](../../../survey/SKILL.md), not by `/change:analyze`. After discovery completes, `/change:survey` runs for every recorded `legacy-code` source between workspace sync and propose. Survey drives the per-language enumeration brief to produce a candidate `surfaces.json`, validates it through `specify change survey`, sizes candidates, applies minimal same-source clustering, and appends candidate blocks under the same `## Candidate inventory` heading in `discovery.md`.

Source-key resolution for legacy-code inputs:

- `--source <k>=<p>` (default kind `legacy-code`) → source-key `<k>`.
- `--source <k>=<p>:legacy-code` (explicit) → source-key `<k>`.
- `--against <p>` (default kind `legacy-code`) → source-key `against`.
- `change.md:inputs[]` with `kind: legacy-code` → source-key `<basename(path) kebab-cased>`.

For a git-URL `--source`, materialise the URL into `legacy/<key>/` with the inlined guarded `git clone` snippet (see [`../../../analyze/SKILL.md` §*Cloning a source tree*](../../../analyze/SKILL.md)) before survey runs.

Documentation-only changes skip `/change:survey` entirely. With no `legacy-code` source, the pipeline reaches propose directly from discovery.

## Merge rule

`/change:analyze` owns append semantics for documentation-derived candidate blocks: dedup-by-name, alphabetic sort, byte-stable output. `/change:survey` owns append semantics for legacy-code-derived candidate blocks. Both skills append under the shared `## Candidate inventory` heading using the unified fenced-YAML candidate block shape defined in [`analyze/SKILL.md` §Output contract](../../../analyze/SKILL.md). This brief invokes analyze once per documentation input in CLI declaration order; the final `discovery.md` inherits each skill's idempotency contract.

Documentation-kind inputs additionally contribute `## Constraints (from documentation)` and `## Open questions (from documentation)` appendix blocks at the end of `discovery.md` (see [`analyze.md` §Documentation branch](./analyze.md) for their shape). Legacy-code candidate blocks are appended by `/change:survey` after discovery completes.

This brief does NOT re-write, re-sort, or re-deduplicate either skill's output; each skill owns its own append contract.

## Output

`discovery.md` has this shape:

````markdown
# Discovery — <change-name>

## Adapter inventory

<!-- source-key: <k> -->
### <adapter-name>

```yaml
summary: <one-sentence imperative description>
sources:
  - <literal artefact path, optionally with fragment>
depends-on: [<other adapter names>]
hints:
  entry_points: [<trigger / command / HTTP verb-path strings>]
  external_deps: [<named external systems>]
confidence: <high | medium | low>
```

<!-- repeat one block per adapter, alphabetically sorted by name -->

## Candidate inventory

<!-- /change:analyze appends documentation-derived candidate blocks here -->
<!-- /change:survey appends legacy-code-derived candidate blocks here -->

## Constraints (from documentation)

- <constraint text> (source: <artifact path[#fragment]>)

## Open questions (from documentation)

- <question text> (source: <artifact path[#fragment]>)
````

Section rules:

- The `# Discovery — <change-name>` header, the `## Adapter inventory` wrapper, and the `## Candidate inventory` heading are written by this brief before invoking `/change:analyze` or `/change:survey`. `/change:analyze` appends its `### <name>` adapter blocks — each preceded by a `<!-- source-key: <k> -->` marker — under `## Adapter inventory`. Both `/change:analyze` and `/change:survey` append candidate blocks under `## Candidate inventory`.
- Every adapter block is emitted by `/change:analyze`; candidate blocks are emitted by either `/change:analyze` (documentation) or `/change:survey` (legacy-code). Both share the unified fenced-YAML candidate block shape pinned in [`analyze/SKILL.md` §Output contract](../../../analyze/SKILL.md).
- The `## Constraints (from documentation)` and `## Open questions (from documentation)` blocks are documentation-branch-only. Omit either heading when empty; never emit an empty section. If no documentation inputs were supplied, both headings are absent.
- A run with only legacy-code inputs produces `## Adapter inventory` and `## Candidate inventory` (candidate blocks appended by `/change:survey` after discovery). A run with only documentation inputs produces `## Adapter inventory`, `## Candidate inventory` (candidate blocks appended by `/change:analyze`), and the two appendix blocks (when non-empty).

## Idempotency

Running discovery twice on the same inputs MUST produce the same `discovery.md`. The contract is inherited from `/change:analyze`:

- Adapters alphabetically sorted by `name`.
- Fixed field order inside each YAML block (`summary`, `sources`, `depends-on`, `hints`, `confidence`).
- `sources`, `depends-on`, `hints.entry_points`, and `hints.external_deps` sorted alphabetically.
- No timestamps, run IDs, working-directory paths, or absolute paths anywhere in the file.
- Re-runs on unchanged sources yield byte-equivalent output; any new detail replaces the prior entry wholesale rather than appending a parallel record.

See [`analyze/SKILL.md` §Idempotency](../../../analyze/SKILL.md) for the authoritative contract.

## Example fragment

See [`../../fixtures/discovery/monolith/expected/discovery.md`](../../fixtures/discovery/monolith/expected/discovery.md) for a single-`--source` legacy-code invocation that produces three adapter summaries (no documentation appendix), and [`../../fixtures/discovery/mixed-inputs/expected/discovery.md`](../../fixtures/discovery/mixed-inputs/expected/discovery.md) for a mixed documentation + legacy-code invocation that produces four adapter summaries plus the documentation appendix blocks.
