---
id: propose
description: Decompose capability inventory into plan entries using Omnia slice heuristics.
needs: [discovery]
generates: .specify/plans/<name>/proposal.md
---

Decompose the capability inventory produced by `discovery.md` into a
concrete set of plan entries, presenting each to the human for
accept/edit/reject review and shelling out to `specify plan create`
for every accepted slice. This is the single-writer edge for
`plan.yaml`: every entry is added via `specify plan create` — the
brief never edits `plan.yaml` directly.

## Input

- `.specify/plans/<name>/discovery.md` (authored by `discovery.md`).
  If the file is missing, stop and report — the discovery brief
  must run first.

## Decomposition heuristics (Omnia)

Omnia is a Rust/WASM stack where the unit of deployment is a WASM
crate (Handler<P> pattern, provider trait bounds). Slice the
inventory with that grain in mind.

1. **One plan entry per WASM crate or handler group.** Never merge
   two independently-deployable crates into a single entry; never
   split a single handler across entries.
2. **Leaf services first.** Favour entries with few dependents
   (small `depends-on` edges). A capability that nothing else
   depends on is a safe first slice; a capability that everything
   depends on is a late slice.
3. **`sources` points at origin.** Each entry's `sources` list
   names the discovery `--source` key (or `against`) the slice
   migrates from; greenfield slices reference the literal artefact
   (`--from`) path as their source.
4. **Cross-cutting refactors are their own entries.** "Extract
   shared validation", "consolidate error types", or similar
   horizontal work becomes a discrete plan entry with explicit
   `depends-on` edges from the feature entries that need it —
   never folded into a feature slice.
5. **Dependency edges follow capability ordering hints.** If
   discovery records "depends on X", the proposed entry inherits a
   `--depends-on X` flag unless X is rejected during review, in
   which case the edge is dropped with a note in `proposal.md`.

## Iteration protocol

For each proposed slice, present the draft to the human and accept
one of three actions:

- **accept** — shell out to:
  ```
  specify plan create <slice-name> \
      --sources <key> [--sources <key>...] \
      --depends-on <preceding> [--depends-on <preceding>...] \
      --affects <area> [--affects <area>...] \
      --description "<one-line summary>"
  ```
  Record the final entry name in the proposal table.
- **edit** — reprompt for the changed field(s) (name, sources,
  depends-on, affects, description) and re-present. Loop until the
  human accepts or rejects.
- **reject** — drop the slice entirely. Later slices that had an
  implicit `depends-on` on this slice lose that edge; if a later
  slice is semantically blocked by the rejection, flag it during
  its own review.

Present slices in the order the heuristics produce (leaves first);
do not re-order mid-loop based on earlier decisions beyond dropping
stale dependency edges.

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
  work, unresolved open questions from discovery>
```

The table MUST include every slice presented to the human — edited
and rejected rows as well as accepted ones — so the proposal
reconstructs the decision trail.

## Final step

After the last accepted slice, run `specify plan validate`. If it
reports any errors, write the error block into the "Notes" section
of `proposal.md` and stop — human triage is required before
execution can begin. Do not attempt to auto-repair plan errors from
within this brief.

## `--dry-run` behaviour

Emit the proposed plan to stdout as a preview of the same table
structure that would be written to `proposal.md`. Do NOT:

- call `specify plan create`,
- write `proposal.md`,
- run `specify plan validate`.

`--dry-run` is read-only; it is safe to invoke repeatedly against
the same discovery output.

## `--extend` behaviour

Skip the `specify plan init` step (the caller, typically the
`/spec:plan` skill, or the human, has already ensured
`.specify/plan.yaml` exists). Still run propose against the
existing plan: slices whose names collide with existing plan
entries are skipped with a note in the proposal; new slices go
through the usual accept/edit/reject loop.

## Example fragment

```markdown
# Proposal — platform-v2

## Slices

| # | Slice              | Source(s) | Depends on          | Decision       | Plan entry         |
|---|--------------------|-----------|---------------------|----------------|--------------------|
| 1 | user-registration  | monolith  | —                   | accept         | user-registration  |
| 2 | email-verification | monolith  | user-registration   | edit → accept  | email-verify       |
| 3 | product-catalog    | monolith  | —                   | accept         | product-catalog    |
| 4 | shopping-cart      | orders    | user-registration   | accept         | shopping-cart      |
| 5 | checkout-api       | payments  | shopping-cart       | accept         | checkout-api       |

## Notes

- Slice 2 renamed from `email-verification` to `email-verify` to
  match the existing module naming convention.
- `specify plan validate` — no errors.
```
