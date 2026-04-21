---
id: propose
description: Map the capability inventory 1:1 onto plan entries and drive the accept/edit/reject/abort loop.
needs: [discovery]
generates: .specify/plans/<name>/proposal.md
---

Turn the capability inventory in `discovery.md` into a concrete set
of plan entries. Decomposition is **mechanical**: one plan entry per
discovered capability. Capability boundaries were decided upstream
by `/spec:analyze`; this brief does not re-cluster. For each
candidate slice, drive the human through an
accept/edit/reject/abort loop and shell out to
`specify initiative create` for every accepted slice. This is the
single-writer edge for `plan.yaml`: every entry is added via
`specify initiative create` — the brief never edits `plan.yaml`
directly.

## Input

- `.specify/plans/<name>/discovery.md` (authored by `discovery.md`).
  If the file is missing, stop and report — the discovery brief
  must run first.
- **`.specify/plans/<name>/workspace.md`** when present (multi-repo /
  Layer 2). Authored by `/spec:plan` step 3(a½) after
  `specify initiative workspace sync`. Summarises each peer under
  `.specify/workspace/<project>/` so propose can attach capabilities
  that land in a peer repo. When absent, assume single-repo mode —
  every `<!-- source-key: <k> -->` MUST resolve to a key in the
  initiative plan's top-level `sources:` map.
- Assumed shape: unified capability summaries as `### <name>`
  headings + fenced YAML (`summary`, `sources`, `depends-on`,
  optional `hints`, `confidence`), each prefixed by a
  `<!-- source-key: <k> -->` HTML comment. Optional trailing
  `## Constraints` and `## Open questions` sections (documentation
  inputs only) are operator context; they do not drive slice
  emission.

## Decomposition — 1:1 capability → slice

`discovery.md` already carries capability boundaries. Propose's job
is to mechanically map each capability to a plan entry. The
clustering judgement is schema-owned inside `/spec:analyze`.

### Mapping rule

For each `### <capability-name>` block:

| Capability field       | Plan entry field                                         |
| ---------------------- | -------------------------------------------------------- |
| `name`                 | `name`                                                   |
| `summary`              | `description` (free-text scoping hint for define)        |
| `<!-- source-key -->`  | `sources: [<key>]` (single-element list)                 |
| `sources:`             | `scope.<key>.include: [...]` (verbatim, alphabetical)    |
| `depends-on:`          | `depends-on: [...]` (verbatim)                           |
| `hints.*`              | Retained in `discovery.md` for operator reference; not carried into `plan.yaml`. |
| `confidence`           | Drives the interactive flag (see §Confidence handling).  |

The `<!-- source-key: <k> -->` HTML comment immediately above each
`### <name>` heading identifies the capability's origin source. Its
value is the plan entry's sole `sources:` entry.

### Peer registry sources (Layer 2)

`workspace.md` is operator-facing context: which peers were synced,
where their `.specify/` trees live under `.specify/workspace/<name>/`,
and whether their checkouts are clean. **Authoring rule for RFC-3a:**
every plan entry MUST still list only `sources:` keys that exist in
the initiative plan's top-level `sources:` map (the single-writer
CLI enforces this today). Use `workspace.md` when deciding *how* to
word `description` / `depends-on` for work that touches shared
contracts across repos; actually pointing a plan entry at a peer
checkout path belongs to **RFC-3b** (federation) — do not invent peer
`scope.*` paths here.

### Documentation capabilities (no source-key marker for code)

Capabilities produced from `/spec:analyze --kind documentation`
carry `sources:` pointing at prose references
(`ops-runbook.md#rotate-upstream-ingest-key`), not code files. The
`<!-- source-key -->` marker still names the documentation input
the capability came from. For these:
- Plan entry `sources:` stays `[<doc-key>]`.
- No `scope.<key>.include` is pre-filled — documentation inputs
  have no extractable file tree to scope.
- `depends-on` still carries over.
- `description` is `[from docs] <summary>` so the operator knows
  the intent source.

### Emit order

Emit in dependency order using `depends-on`: leaves first,
transitive dependents later. Within a layer, emit alphabetically by
`name`. This mirrors the topological order `specify initiative next`
walks at execution time.

### Confidence handling

- `confidence: high` / `medium` → ordinary candidate in the
  accept/edit/reject/abort loop.
- `confidence: low` → surface with a **⚠ review before accepting**
  flag on the first line of the prompt. The flag is advisory; it
  never auto-rejects. Low-confidence capabilities are where
  clustering was least certain — typical triggers for a rename, a
  scope edit, or (at Stage C) a manifest pointer.

### Tangled / overlapping capabilities

Where two capabilities' `sources:` lists overlap (the same file
path appears under more than one capability on the same source
key), the default is still **glob-based** emission: verbatim
per-file paths in `scope.<key>.include` and one `--scope-include`
per hint, same as non-overlapping `high` / `medium` capabilities.
`specify initiative validate` surfaces the overlap as a
`scope-overlap` warning (RFC-3a §*Validation*); the human may
narrow scope, split the shared file, or defer cleanup during the
accept/edit/reject loop.

**Stage C — manifest escape hatch.** When **`confidence: low`**
*and* either (a) that capability's file hints overlap another
capability's `sources:` on the same path for the same source key,
or (b) a clean 1:1 mapping from `sources:` entries to
`--scope-include` globs is ambiguous, the brief MUST **not** rely
on repeated `--scope-include` for that source key on the affected
slice. Instead it MUST:

1. Write a v1 slice manifest to
   `.specify/plans/<initiative>/slices/<change-name>.yaml` (`version:
   1` and `include:` — each path relative to that source key's
   root in the plan's `sources` map), enumerating the exact files
   for extraction.
2. Shell out to `specify initiative create` with
   **`--scope-manifest <source-key>=<project-relative-path-to-yaml>`**
   (exactly once per scoped source key on that invocation) instead
   of multiple `--scope-include` flags for that key.

Shape and validation errors (`manifest-invalid`, `manifest-empty`,
`manifest-path-escape`, etc.) match `/spec:extract` §*Manifest
shape* ([`plugins/spec/skills/extract/SKILL.md`](../../../../plugins/spec/skills/extract/SKILL.md#manifest-shape))
and `specify initiative validate` in `specify-cli`.

## Omnia carry-through

Capability names flow directly into change names; the
one-WASM-crate-per-change convention is preserved at
`/spec:define` time, not here. No grouping, no renaming, no
cross-capability merges in this brief — edits happen through the
interactive loop, one slice at a time.

## `specify initiative create` invocation

For each accepted slice, shell out once:

```text
specify initiative create <name> \
    --sources <source-key> \
    --depends-on <dep1> [--depends-on <dep2> ...] \
    --scope-include <source-key>=<glob1> \
    [--scope-include <source-key>=<glob2> ...] \
    [--scope-manifest <source-key>=<project-relative-yaml> ...] \
    --description "<summary>"
```

- One `--scope-include` flag per file-hint, verbatim, when Stage C
  does not require a manifest for that source key (see §*Tangled /
  overlapping capabilities*). Per-file globs are the 1:1 default;
  the operator may widen to a directory glob (`src/users/**`)
  during edit.
- **`--scope-manifest <source-key>=<path>`** — mutually exclusive
  with `--scope-include` / `--scope-exclude` for the same source
  key on one invocation. Use exactly one flag per affected key when
  the brief emits a slice manifest (Stage C); `<path>` is relative
  to the project root (the same path stored in
  `plan.yaml` as `scope.<key>.manifest`).
- Omit `--scope-include` entirely for documentation capabilities
  (no file tree to scope).
- `--scope-exclude` is not emitted automatically; it is reachable
  through edit. `--scope-manifest` is emitted only under the Stage C
  rules in §*Tangled / overlapping capabilities*.

## Interactive loop

For each candidate slice in emit order:

1. Present **name** + schema-canonical `summary`.
2. Show **sources** (source key) and **scope.include**
   (file-hint list).
3. Show **depends-on** graph preview.
4. If `confidence: low`, prepend **⚠ review before accepting** to
   the first line of the prompt.
5. Accept one of four actions:
   - **accept** — shell out to `specify initiative create` with
     the mapped flags above. Record the entry in the proposal
     table.
   - **edit** — reprompt for changed field(s) (name, sources,
     depends-on, scope-include, scope-exclude, scope-manifest,
     description) and re-present. Loop until accept or reject.
     Edits may widen or narrow scope globs, rename the capability,
     drop a dependency edge, switch a per-file glob to a directory
     glob, or move between glob scope and a slice manifest (Stage C).
   - **reject** — drop the slice. Upcoming slices with an
     implicit `depends-on` on this slice lose that edge before
     they are presented; if a later slice is semantically blocked
     by the rejection, flag it during its own review.
   - **abort** — stop the loop. Already-accepted entries remain
     on disk (written by `specify initiative create`); the brief
     writes `proposal.md` with decisions to date and exits
     non-zero, pointing the operator at `/spec:plan --extend` to
     resume.

Present slices in the order the emit rule produces; do not
re-order mid-loop beyond dropping stale dependency edges after a
reject.

## Output

Write `.specify/plans/<name>/proposal.md` regardless of per-slice
decisions — the proposal is the audit trail of the authoring run.
Shape:

```markdown
# Proposal — <initiative-name>

## Slices

| # | Slice | Source(s) | Depends on | Decision | Plan entry |
|---|---|---|---|---|---|
| 1 | <proposed name> | <keys> | <slice names or —> | accept | <final name> |
| 2 | ... | ... | ... | edit → accept | <final name> |
| 3 | ... | ... | ... | reject | — |

## Notes

- <free-form notes: why slices were edited, why rejected, deferred
  work, scope-overlap warnings seen, unresolved open questions
  from discovery>
```

The table MUST include every slice presented to the human — edited
and rejected rows as well as accepted ones — so the proposal
reconstructs the decision trail.

## Final step

After the last accepted slice, run `specify initiative validate`.
If it reports errors, write the error block into the "Notes"
section of `proposal.md` and stop — human triage is required
before execution can begin. `scope-overlap` warnings (see
§Tangled / overlapping capabilities) are copied into "Notes" but
do not block completion. Do not attempt to auto-repair plan
errors from within this brief.

## Example — monolith fixture

Given
[`plugins/spec/skills/plan/fixtures/discovery/monolith/expected/discovery.md`](../../../../plugins/spec/skills/plan/fixtures/discovery/monolith/expected/discovery.md):

- `email-verification` (sources: `src/auth/verify.ts`; depends-on: —)
- `shared-validation` (sources: `src/common/validation.ts`; depends-on: —)
- `user-registration` (sources: `src/auth/verify.ts`, `src/users/register.ts`, `src/users/validation.ts`; depends-on: `[email-verification, shared-validation]`)

Propose emits (dependency order, alphabetical within layer):

1. ```text
   specify initiative create email-verification \
       --sources monolith \
       --scope-include monolith=src/auth/verify.ts \
       --description "Verify a newly registered account via a one-time email token."
   ```
2. ```text
   specify initiative create shared-validation \
       --sources monolith \
       --scope-include monolith=src/common/validation.ts \
       --description "Validate common user-facing inputs with reusable primitives."
   ```
3. ```text
   specify initiative create user-registration \
       --sources monolith \
       --depends-on email-verification --depends-on shared-validation \
       --scope-include monolith=src/auth/verify.ts \
       --scope-include monolith=src/users/register.ts \
       --scope-include monolith=src/users/validation.ts \
       --description "Create new user accounts with email verification."
   ```

`src/auth/verify.ts` appears under two capabilities — an
intentional overlap. `specify initiative validate` surfaces it as a
`scope-overlap` warning; the human resolves during the loop (the
usual fix is to narrow `user-registration`'s scope to
`src/users/**` on the edit step).

## `--dry-run` behaviour

Emit the proposed plan to stdout as a preview of the same table
structure that would be written to `proposal.md`. Do NOT:

- call `specify initiative create`,
- write `proposal.md`,
- run `specify initiative validate`.

`--dry-run` is read-only; it is safe to invoke repeatedly against
the same discovery output.

## `--extend` behaviour

Skip the `specify initiative init` step (the caller — typically
the `/spec:plan` skill — has already ensured `.specify/plan.yaml`
exists). Still run propose against the existing plan: slices whose
names collide with existing plan entries are skipped with a note
in the proposal; new slices go through the usual loop.
