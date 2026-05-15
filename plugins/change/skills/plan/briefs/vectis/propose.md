---
id: propose
description: "Decompose inventory into plan entries. Vectis heuristic: shared-core first, per-shell last."
needs: [discovery]
generates: .specify/plans/<name>/proposal.md
---

Decompose the capability inventory produced by `discovery.md` into a concrete set of plan entries, presenting each to the human for accept/edit/reject review and shelling out to `specify change plan add` for every accepted slice. This is the propose edge of the shared [plan single-writer contract](../../../../references/plan-single-writer.md): entries are added without `--project`, and project assignment is handled by the plan skill's assignment step (RFC-3b), not by this brief.

## Input

- `.specify/plans/<name>/discovery.md` (authored by `discovery.md`). If the file is missing, stop and report — the discovery brief must run first.
- **`.specify/plans/<name>/workspace.md`** when present (multi-repo). Authored by `/change:plan` step 3(b) after `specify workspace sync`. Summarises each peer under `.specify/workspace/<project>/` so propose can attach capabilities that land in a peer repo. When absent, assume single-repo mode.

## Decomposition heuristics (Vectis)

Vectis is a Crux stack: one Rust shared core crate with an `App` trait that is consumed by a SwiftUI iOS shell and a Jetpack Compose Android shell. Each shell reads `tokens.yaml` and `assets.yaml` directly and emits its own theme + asset code under its own tree (no shared design-system library — RFC-11 §L). Slice the inventory with that grain in mind.

1. **Shared-core first.** Every capability classified as shared core under `discovery.md`'s `### Shared core` tier becomes a plan entry before any shell entry. Shared core is the dependency backbone: shell views bind to it, so shelving the cores earlier in the plan keeps `depends-on` edges forward and avoids dependent slices blocking on missing `App` traits. Name shared-core entries with a `-core` suffix (`counter-core`, `theme-core`) to make the tier obvious at a glance in `specify change plan status`.
2. **UI inputs are slices only when independently reviewable.** Token, asset, and layout work (`tokens.yaml`, `assets.yaml`, `layout.yaml`) is *input context* for the shells, not a peer tier (RFC-11 §L). Discovery surfaces these capabilities under `## Cross-cutting UI inputs`; propose creates a plan entry for one **only** when the work is large enough to warrant its own review pass — for example, "migrate legacy SCSS into `tokens.yaml`" or "author `assets.yaml` from a Figma asset export". Trivially-coupled token / asset edits (a single new colour added in service of a single new shell view) stay folded into the consuming shell entry rather than becoming their own slice. When a UI-input slice IS created, present it after the shared-core entries it reads from (if any) and before the shell entries that consume it; name it after the artifact it produces (`design-tokens` for `tokens.yaml`, `app-icons` for `assets.yaml`, `<screen>-layout` for `layout.yaml`) so the artifact destination is obvious. There is no longer a default "design-tokens" rung between core and shells — the slice appears only when discovery surfaced a UI-input capability **and** the operator confirmed during accept/edit/reject that it warrants an independent review pass.
3. **Per-shell last.** Every iOS-shell and Android-shell capability from `discovery.md` becomes its own plan entry, presented *after* the shared-core entries it depends on (and after any UI-input slices that were promoted under heuristic 2). The default `depends-on` edges for a shell entry are the corresponding shared-core entry PLUS every promoted UI-input slice the shell consumes, seeded from discovery's ordering hints. When a UI-input capability surfaced in discovery but was NOT promoted to its own plan entry (heuristic 2), the shell entry that consumes it carries the UI-input edit inline — no `depends-on` edge is added for it because the work happens within the shell slice itself. Name shell entries with a platform-suffixed `-ios-view` / `-android-view` / `-ios-binding` / `-android-binding` to make the platform obvious at a glance.
4. **One plan entry per Crux unit.** The Crux units are:
   - one shared-core `App` trait family,
   - one SwiftUI view or binding per iOS capability,
   - one Compose view or binding per Android capability,
   - optionally, one UI-input slice per artifact (`tokens.yaml`, `assets.yaml`, or `layout.yaml`) when heuristic 2 promotes the work.
   Never merge two independently-shippable shell views into a single entry; never split a single `App` trait across entries; never bundle two distinct UI-input artifacts (e.g. `tokens.yaml` AND `assets.yaml`) into a single slice — they are validated and reviewed independently by `specify tool run vectis -- validate tokens` / `assets`.
5. **iOS and Android shells are siblings.** iOS-shell and Android-shell entries never depend on each other directly — both depend on the same shared-core ancestors plus any UI-input slices that were promoted under heuristic 2. Present iOS entries before Android in the draft order (alphabetical tie-break by capability name), but dependency edges never cross the two shells.
6. **Cross-cutting refactors are their own entries.** "Extract a shared ViewModel adapter", "consolidate `Command` error types", or similar horizontal work becomes a discrete plan entry with explicit `depends-on` edges from the feature entries that need it — never folded into a feature slice. Present cross-cutting entries immediately before the feature entries that seed their `depends-on` edges so rejects only trim drafts (never already-written entries).
7. **Dependency edges follow capability ordering hints.** If discovery records "depends on X" or "consumes X" on a capability, the proposed entry inherits a `--depends-on X` flag unless X is rejected during review, in which case the edge is dropped with a note in `proposal.md`.
8. **`sources` points at origin.** Each entry's `sources` list names the discovery `--source` key (or `against`) the slice migrates from; greenfield slices reference the literal artefact (`--from`) path as their source. A shared-core entry typically carries every source that mentioned the capability under the `### Shared core` tier; shell entries carry only the source(s) for their specific platform.

### Peer registry sources (multi-repo)

Project assignment is handled by the plan skill's assignment step (RFC-3b §*Assignment algorithm*), not by the propose brief. The propose brief creates entries without `--project`. `workspace.md` is operator-facing context: which peers were synced, where their `.specify/` trees live under `.specify/workspace/<name>/`, and whether their checkouts are clean. **Authoring rule:** every plan entry MUST still list only `sources:` keys that exist in the change plan's top-level `sources:` map (the single-writer CLI enforces this today).

When the assignment step (3(d)) routes an entry to a project that does not yet exist in `registry.yaml`, the plan skill — not this brief — runs the **registry-proposal sub-step** (RFC-9 §2B; see `plugins/change/skills/plan/SKILL.md` → §"Step 3(d).1 — Registry proposal sub-step"). The sub-step shells out to `specify registry add`, then `specify workspace sync`, then `specify change plan amend --project <name>` for the entry. This brief never proposes registry entries directly.

### Resulting draft order

For a two-platform change with two shared-core capabilities and matching iOS + Android views, with no independently-reviewable UI-input work in scope, the heuristic produces the following draft order:

```text
1. <name>-core                    (shared core; leaves first)
2. <other>-core                   (shared core)
3. <name>-ios-view                (iOS shell; depends-on: <name>-core)
4. <other>-ios-binding            (iOS shell; depends-on: <other>-core)
5. <name>-android-view            (Android shell; depends-on: <name>-core)
6. <other>-android-binding        (Android shell; depends-on: <other>-core)
```

When discovery surfaced a UI-input capability that the operator promotes under heuristic 2 (e.g. a legacy `tokens.yaml` migration or an `assets.yaml` import from a Figma export), the slice slots in between the shared-core entries it reads from and the shell entries that consume it:

```text
1. <name>-core                    (shared core)
2. theme-core                     (shared core)
3. design-tokens                  (UI input — tokens.yaml; depends-on: theme-core)
4. <name>-ios-view                (iOS shell; depends-on: <name>-core, design-tokens)
5. <name>-android-view            (Android shell; depends-on: <name>-core, design-tokens)
```

The human can edit any slice's `name`, `sources`, `depends-on`, or `description` during the iteration protocol — the heuristic seeds the draft; the review refines it.

## Iteration protocol

For each proposed slice, present the draft to the human and accept one of three actions:

- **accept** — shell out to:
  ```
  specify change plan add <slice-name> \
      --sources <key> [--sources <key>...] \
      --depends-on <preceding> [--depends-on <preceding>...] \
      --description "<rich description with delta-targeting intent>"
  ``` Record the final entry name in the proposal table.
- **edit** — reprompt for the changed field(s) (name, sources, depends-on, description) and re-present. Loop until the human accepts or rejects.
- **reject** — drop the slice entirely. Later slices that had an implicit `depends-on` on this slice lose that edge; if a later slice is semantically blocked by the rejection (e.g. a shell view whose shared-core was just rejected), flag it during its own review so the human decides whether to reject the dependent shell too or carry on.

Present slices in the order the heuristics produce (shared-core first, then any promoted UI-input slices in artifact-name order, then per-shell, alphabetical within each tier); do not re-order mid-loop based on earlier decisions beyond dropping stale dependency edges.

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

After the last accepted slice, run `specify change plan validate`. If it reports any errors, write the error block into the "Notes" section of `proposal.md` and stop — human triage is required before execution can begin. Do not attempt to auto-repair plan errors from within this brief.

## `--dry-run` behaviour

Emit the proposed plan to stdout as a preview of the same table structure that would be written to `proposal.md`. Do NOT:

- call `specify change plan add`,
- write `proposal.md`,
- run `specify change plan validate`.

`--dry-run` is read-only; it is safe to invoke repeatedly against the same discovery output.

## `--extend` behaviour

Skip the `specify change create` scaffold step (the caller, typically the `/change:plan` skill, or the human, has already ensured `change.md` and `plan.yaml` exist). Still run propose against the existing plan: slices whose names collide with existing plan entries are skipped with a note in the proposal; new slices go through the usual accept/edit/reject loop.

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
  `plugins/change/skills/plan/briefs/vectis/propose.md`): shared-core first
  (slices 1–2). The `design-tokens` UI-input slice was promoted
  under heuristic 2 — the legacy SCSS palette is large enough to
  warrant an independent review pass, so it was sliced rather
  than folded into the shell entries — and slotted between core
  and shells (slice 3). Per-shell last (slices 4–5); each shell
  slice depends on the corresponding `-core` entry plus the
  promoted `design-tokens` input.
- Slice 5's `description` was edited during review to clarify
  the Compose binding is Material 3.
- `specify change plan validate` — no errors.
```
