---
id: propose
description: Map the capability inventory 1:1 onto plan entries and drive the accept/edit/reject/abort loop.
needs: [discovery]
generates: .specify/plans/<name>/proposal.md
---

Turn the capability inventory in `discovery.md` into a concrete set of plan entries. Decomposition is **mechanical**: one plan entry per discovered capability. Capability boundaries were decided upstream by `/spec:analyze`; this brief does not re-cluster. For each candidate slice, drive the human through an accept/edit/reject/abort loop and shell out to `specify change plan add` for every accepted slice. This is the propose edge of the shared [plan single-writer contract](../../../../references/plan-single-writer.md): entries are added without `--project`, and project assignment is handled by the plan skill's assignment step (RFC-3b), not by this brief.

## Input

- `.specify/plans/<name>/discovery.md` (authored by `discovery.md`). If the file is missing, stop and report — the discovery brief must run first.
- **`.specify/plans/<name>/workspace.md`** when present (multi-repo). Authored by `/change:plan` step 3(b) after `specify workspace sync`. Summarises each peer under `.specify/workspace/<project>/` so propose can attach capabilities that land in a peer repo. When absent, assume single-repo mode — every `<!-- source-key: <k> -->` MUST resolve to a key in the change plan's top-level `sources:` map.
- Assumed shape: unified capability summaries as `### <name>` headings + fenced YAML (`summary`, `sources`, `depends-on`, optional `hints`, `confidence`), each prefixed by a `<!-- source-key: <k> -->` HTML comment. Optional trailing `## Constraints` and `## Open questions` sections (documentation inputs only) are operator context; they do not drive slice emission.

## Decomposition — 1:1 capability → slice

`discovery.md` already carries capability boundaries. Propose's job is to mechanically map each capability to a plan entry. The clustering judgement is capability-owned inside `/spec:analyze`.

### Mapping rule

For each `### <capability-name>` block:

| Capability field       | Plan entry field                                         |
| ---------------------- | -------------------------------------------------------- |
| `name`                 | `name`                                                   |
| `summary` + `sources:` | `description` (rich prose — see §Rich description generation) |
| `<!-- source-key -->`  | `sources: [<key>]` (single-element list)                 |
| `depends-on:`          | `depends-on: [...]` (verbatim)                           |
| `hints.*`              | Retained in `discovery.md` for operator reference; not carried into `plan.yaml`. |
| `confidence`           | Drives the interactive flag (see §Confidence handling).  |

The `<!-- source-key: <k> -->` HTML comment immediately above each `### <name>` heading identifies the capability's origin source. Its value is the plan entry's sole `sources:` entry.

### Peer registry sources (multi-repo)

Project assignment is handled by the plan skill's assignment step (RFC-3b §*Assignment algorithm*), not by the propose brief. The propose brief creates entries without `--project`. `workspace.md` is operator-facing context: which peers were synced, where their `.specify/` trees live under `.specify/workspace/<name>/`, and whether their checkouts are clean. **Authoring rule:** every plan entry MUST still list only `sources:` keys that exist in the change plan's top-level `sources:` map (the single-writer CLI enforces this today).

When the assignment step (3(d)) routes an entry to a project that does not yet exist in `registry.yaml`, the plan skill — not this brief — runs the **registry-proposal sub-step** (RFC-9 §2B; see `plugins/change/skills/plan/SKILL.md` → §"Step 3(d).1 — Registry proposal sub-step"). The sub-step shells out to `specify registry add`, then `specify workspace sync`, then `specify change plan amend --project <name>` for the entry. This brief never proposes registry entries directly.

### Documentation capabilities (no source-key marker for code)

Capabilities produced from `/spec:analyze documentation` carry `sources:` pointing at prose references (`ops-runbook.md#rotate-upstream-ingest-key`), not code files. The `<!-- source-key -->` marker still names the documentation input the capability came from. For these:
- Plan entry `sources:` stays `[<doc-key>]`.
- `depends-on` still carries over.
- `description` is `[from docs] <summary>` so the operator knows the intent source. No file-path hints are included since documentation inputs have no extractable file tree.

### Emit order

Emit in dependency order using `depends-on`: leaves first, transitive dependents later. Within a layer, emit alphabetically by `name`. This mirrors the topological order `specify change plan next` walks at execution time.

### Rich description generation

The `description` field carries all scoping and delta-targeting intent as free-form prose. For each capability, assemble the description from these inputs:

1. **Capability summary** — the `summary` text from discovery, forming the opening sentence(s).
2. **File-path hints** — if the capability's `sources:` list contains file paths, append a sentence such as "Focus on `src/common/validation/`." or "Relevant files: `src/auth/verify.ts`, `src/users/register.ts`." Use directory prefixes when multiple files share a common parent; use individual paths when the list is short (≤ 3 entries).
3. **Delta-targeting intent** — when the capability overlaps with a prior baseline (an existing spec set from a merged change), append "Delta-targets `<prior-change-name>`." so the define brief knows to produce deltas, not a full extraction.
4. **Scope-narrowing language** — incorporate any narrowing hints from the discovery phase's `hints` or `constraints` (e.g. "Excludes legacy migration paths." or "Limited to the v2 API surface.").

The generated description is presented to the operator in the interactive loop and can be refined during an edit action.

### Confidence handling

- `confidence: high` / `medium` → ordinary candidate in the accept/edit/reject/abort loop.
- `confidence: low` → surface with a **⚠ review before accepting** flag on the first line of the prompt. The flag is advisory; it never auto-rejects. Low-confidence capabilities are where clustering was least certain — typical triggers for a rename or a description edit.

## Omnia carry-through

Capability names flow directly into change names; the one-WASM-crate-per-slice convention is preserved at `/spec:define` time, not here. No grouping, no renaming, no cross-capability merges in this brief — edits happen through the interactive loop, one slice at a time.

## `specify change plan add` invocation

For each accepted slice, shell out once:

```text
specify change plan add <name> \
    --sources <source-key> \
    --depends-on <dep1> [--depends-on <dep2> ...] \
    --description "<rich prose>"
```

- `--description` carries the rich prose generated per §Rich description generation — file-path hints, delta-targeting intent, and scope-narrowing language all live here.

## Interactive loop

For each candidate slice in emit order:

1. Present **name** + generated `description` (rich prose).
2. Show **sources** (source key).
3. Show **depends-on** graph preview.
4. If `confidence: low`, prepend **⚠ review before accepting** to the first line of the prompt.
5. Accept one of four actions:
   - **accept** — shell out to `specify change plan add` with the mapped flags above. Record the entry in the proposal table.
   - **edit** — reprompt for changed field(s) (name, sources, depends-on, description) and re-present. Loop until accept or reject. Edits may rename the capability, drop a dependency edge, or refine the description prose.
   - **reject** — drop the slice. Upcoming slices with an implicit `depends-on` on this slice lose that edge before they are presented; if a later slice is semantically blocked by the rejection, flag it during its own review.
   - **abort** — stop the loop. Already-accepted entries remain on disk (written by `specify change plan add`); the brief writes `proposal.md` with decisions to date and exits non-zero, pointing the operator at `/change:plan <name> extend` to resume.

Present slices in the order the emit rule produces; do not re-order mid-loop beyond dropping stale dependency edges after a reject.

## Output

Write `.specify/plans/<name>/proposal.md` regardless of per-slice decisions — the proposal is the audit trail of the authoring run. Shape:

```markdown
# Proposal — <change-name>

## Slices

| # | Slice | Source(s) | Depends on | Decision | Plan entry |
|---|---|---|---|---|---|
| 1 | <proposed name> | <keys> | <slice names or —> | accept | <final name> |
| 2 | ... | ... | ... | edit → accept | <final name> |
| 3 | ... | ... | ... | reject | — |

## Notes

- <free-form notes: why slices were edited, why rejected, deferred
  work, unresolved open questions from discovery>
```

The table MUST include every slice presented to the human — edited and rejected rows as well as accepted ones — so the proposal reconstructs the decision trail.

## Final step

After the last accepted slice, run `specify change plan validate`. If it reports errors, write the error block into the "Notes" section of `proposal.md` and stop — human triage is required before execution can begin. Do not attempt to auto-repair plan errors from within this brief.

## Example — monolith fixture

Given [`plugins/change/skills/plan/fixtures/discovery/monolith/expected/discovery.md`](../../fixtures/discovery/monolith/expected/discovery.md):

- `email-verification` (sources: `src/auth/verify.ts`; depends-on: —)
- `shared-validation` (sources: `src/common/validation.ts`; depends-on: —)
- `user-registration` (sources: `src/auth/verify.ts`, `src/users/register.ts`, `src/users/validation.ts`; depends-on: `[email-verification, shared-validation]`)

Propose emits (dependency order, alphabetical within layer):

1. ```text specify change plan add email-verification \
       --sources monolith \
       --description "Verify a newly registered account via a one-time email token. Focus on src/auth/verify.ts."
   ```
2. ```text
   specify change plan add shared-validation \
       --sources monolith \
       --description "Validate common user-facing inputs with reusable primitives. Focus on src/common/validation/."
   ```
3. ```text specify change plan add user-registration \
       --sources monolith \
       --depends-on email-verification --depends-on shared-validation \
       --description "Create new user accounts with email verification. Relevant files: src/auth/verify.ts, src/users/register.ts, src/users/validation.ts. Delta-targets email-verification."
   ```

`src/auth/verify.ts` appears under two capabilities.
`user-registration`'s description carries a delta-targeting hint
(`Delta-targets email-verification.`) so the define brief knows
to produce deltas against the already-extracted baseline from
`email-verification`. The operator may refine the description
prose during the edit step (e.g. narrowing to
"Focus on `src/users/`.").

## `--dry-run` behaviour

Emit the proposed plan to stdout as a preview of the same table
structure that would be written to `proposal.md`. Do NOT:

- call `specify change plan add`,
- write `proposal.md`,
- run `specify change plan validate`.

`--dry-run` is read-only; it is safe to invoke repeatedly against
the same discovery output.

## `--extend` behaviour

Skip the `specify change plan create` step (the caller — typically
the `/change:plan` skill — has already ensured `plan.yaml`
exists). Still run propose against the existing plan: slices whose
names collide with existing plan entries are skipped with a note
in the proposal; new slices go through the usual loop.
