---
id: propose
description: Decompose inventory into plan entries. Vectis heuristic: shared-core first, per-shell last.
needs: [discovery]
generates: .specify/plans/<name>/proposal.md
---

Decompose the capability inventory produced by `discovery.md` into a
concrete set of plan entries, presenting each to the human for
accept/edit/reject review and shelling out to `specify initiative create`
for every accepted slice. This is the single-writer edge for
`plan.yaml`: every entry is added via `specify initiative create` — the
brief never edits `plan.yaml` directly.

## Input

- `.specify/plans/<name>/discovery.md` (authored by `discovery.md`).
  If the file is missing, stop and report — the discovery brief
  must run first.

## Decomposition heuristics (Vectis)

Vectis is a Crux stack: one Rust shared core crate with an `App`
trait that is consumed by a SwiftUI iOS shell and a Jetpack
Compose Android shell, with design tokens generating a Swift
Package on iOS and an Android `vectis-design` Compose library.
Slice the inventory with that grain in mind.

1. **Shared-core first.** Every capability classified as shared
   core under `discovery.md`'s `### Shared core` tier becomes a
   plan entry before any shell entry. Shared core is the
   dependency backbone: shell views bind to it, so shelving the
   cores earlier in the plan keeps `depends-on` edges forward and
   avoids dependent slices blocking on missing `App` traits. Name
   shared-core entries with a `-core` suffix (`counter-core`,
   `theme-core`) to make the tier obvious at a glance in
   `specify initiative status`.
2. **Design system next.** Design-tokens and shared component
   primitives land after the shared-core entries they read from
   (e.g. `design-tokens` depends on `theme-core`) and before any
   shell that consumes them. On Vectis this is usually a single
   entry named `design-tokens`, regenerated for both platforms by
   `/vectis:design-system-writer`; split only when the inventory
   surfaces distinct token families (e.g. `design-tokens-colour`
   vs `design-tokens-typography`) that ship independently.
3. **Per-shell last.** Every iOS-shell and Android-shell
   capability from `discovery.md` becomes its own plan entry,
   presented *after* the shared-core and design-system entries
   it depends on. The default `depends-on` edges for a shell
   entry are the corresponding shared-core entry PLUS every
   design-system entry the shell consumes, seeded from
   discovery's ordering hints. Name shell entries with a
   platform-suffixed `-ios-view` / `-android-view` /
   `-ios-binding` / `-android-binding` to make the platform
   obvious at a glance.
4. **One plan entry per Crux unit.** The Crux units are:
   - one shared-core `App` trait family,
   - one design-tokens regeneration pass (or N if the inventory
     splits tokens),
   - one SwiftUI view or binding per iOS capability,
   - one Compose view or binding per Android capability.
   Never merge two independently-shippable shell views into a
   single entry; never split a single `App` trait across entries.
5. **iOS and Android shells are siblings.** iOS-shell and
   Android-shell entries never depend on each other directly —
   both depend on the same shared-core + design-tokens
   ancestors. Present iOS entries before Android in the draft
   order (alphabetical tie-break by capability name), but
   dependency edges never cross the two shells.
6. **Cross-cutting refactors are their own entries.** "Extract a
   shared ViewModel adapter", "consolidate `Command` error
   types", or similar horizontal work becomes a discrete plan
   entry with explicit `depends-on` edges from the feature
   entries that need it — never folded into a feature slice.
   Present cross-cutting entries immediately before the feature
   entries that seed their `depends-on` edges so rejects only
   trim drafts (never already-written entries).
7. **Dependency edges follow capability ordering hints.** If
   discovery records "depends on X" or "consumes X" on a
   capability, the proposed entry inherits a `--depends-on X`
   flag unless X is rejected during review, in which case the
   edge is dropped with a note in `proposal.md`.
8. **`sources` points at origin.** Each entry's `sources` list
   names the discovery `--source` key (or `against`) the slice
   migrates from; greenfield slices reference the literal
   artefact (`--from`) path as their source. A shared-core entry
   typically carries every source that mentioned the capability
   under the `### Shared core` tier; shell entries carry only
   the source(s) for their specific platform.

### Resulting draft order

For a two-platform initiative with one shared-core capability,
one design-system capability, and matching iOS + Android views,
the heuristic produces the following draft order:

```text
1. <name>-core                    (shared core; leaves first)
2. <other>-core                   (shared core)
3. design-tokens                  (design system; depends-on: <related core>)
4. <name>-ios-view                (iOS shell; depends-on: <name>-core, design-tokens)
5. <other>-ios-binding            (iOS shell; depends-on: <other>-core, design-tokens)
6. <name>-android-view            (Android shell; depends-on: <name>-core, design-tokens)
7. <other>-android-binding        (Android shell; depends-on: <other>-core, design-tokens)
```

The human can edit any slice's `name`, `sources`, `depends-on`,
or `description` during the iteration protocol — the heuristic
seeds the draft; the review refines it.

## Iteration protocol

For each proposed slice, present the draft to the human and
accept one of three actions:

- **accept** — shell out to:
  ```
  specify initiative create <slice-name> \
      --sources <key> [--sources <key>...] \
      --depends-on <preceding> [--depends-on <preceding>...] \
      --description "<rich description with delta-targeting intent>"
  ```
  Record the final entry name in the proposal table.
- **edit** — reprompt for the changed field(s) (name, sources,
  depends-on, description) and re-present. Loop until
  the human accepts or rejects.
- **reject** — drop the slice entirely. Later slices that had an
  implicit `depends-on` on this slice lose that edge; if a later
  slice is semantically blocked by the rejection (e.g. a shell
  view whose shared-core was just rejected), flag it during its
  own review so the human decides whether to reject the
  dependent shell too or carry on.

Present slices in the order the heuristics produce (shared-core
first, then design system, then per-shell, alphabetical within
each tier); do not re-order mid-loop based on earlier decisions
beyond dropping stale dependency edges.

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

The table MUST include every slice presented to the human —
edited and rejected rows as well as accepted ones — so the
proposal reconstructs the decision trail.

## Final step

After the last accepted slice, run `specify initiative validate`. If it
reports any errors, write the error block into the "Notes" section
of `proposal.md` and stop — human triage is required before
execution can begin. Do not attempt to auto-repair plan errors
from within this brief.

## `--dry-run` behaviour

Emit the proposed plan to stdout as a preview of the same table
structure that would be written to `proposal.md`. Do NOT:

- call `specify initiative create`,
- write `proposal.md`,
- run `specify initiative validate`.

`--dry-run` is read-only; it is safe to invoke repeatedly against
the same discovery output.

## `--extend` behaviour

Skip the `specify initiative init` step (the caller, typically the
`/spec:plan` skill, or the human, has already ensured
`.specify/plan.yaml` exists). Still run propose against the
existing plan: slices whose names collide with existing plan
entries are skipped with a note in the proposal; new slices go
through the usual accept/edit/reject loop.

## Example fragment

```markdown
# Proposal — counter-migration

## Slices

| # | Slice                 | Source(s)                    | Depends on                    | Decision      | Plan entry            |
|---|-----------------------|------------------------------|-------------------------------|---------------|-----------------------|
| 1 | counter-core          | legacy-ios, legacy-android   | —                             | accept        | counter-core          |
| 2 | theme-core            | legacy-ios, legacy-android   | —                             | accept        | theme-core            |
| 3 | design-tokens         | legacy-tokens                | theme-core                    | accept        | design-tokens         |
| 4 | counter-ios-view      | legacy-ios                   | counter-core, design-tokens   | accept        | counter-ios-view      |
| 5 | counter-android-view  | legacy-android               | counter-core, design-tokens   | edit → accept | counter-android-view  |

## Notes

- Heuristics applied (Vectis, from
  `schemas/vectis/briefs/plan/propose.md`): shared-core first
  (slices 1–2), design-system next (slice 3), per-shell last
  (slices 4–5). Each shell slice depends on the corresponding
  `-core` entry plus `design-tokens`.
- Slice 5's `description` was edited during review to clarify
  the Compose binding is Material 3.
- `specify initiative validate` — no errors.
```
