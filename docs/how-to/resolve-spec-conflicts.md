# Resolve spec conflicts

Handle `[conflict]` and `[divergence]` tags after `emery plan refine` synthesizes `spec.md`.

**Prerequisites:** A refined slice with tagged requirements; read [Core concepts](../explanation/concepts.md) § Evidence, provenance, authority.

## Understand the tags

| Tag | Meaning | Slice blocked? |
| --- | ------- | -------------- |
| `[divergence]` | Higher-authority source won; loser preserved as commentary | No — refine still reaches `refined` |
| `[conflict]` | Same-authority sources disagree; no automatic winner | No — operator reconciles |
| `[unknown]` | Insufficient evidence | No — mark for follow-up |

Tags appear on requirement headers alongside `Status: conflict`, `Status: divergence`, or `Status: unknown`.

## The one rule

The `ID:` / `Sources:` / `Status:` lines and the `[conflict]` / `[divergence]` / `[unknown]` headline tags are **kernel-rendered** — written by the CLI's deterministic projection kernel from `model.yaml`, not by you or the agent — so **never hand-edit them**. The provenance parser (run by `emery slice validate` and the refine gate) refuses any edit that stales a kernel line and reports `slice-spec-provenance-stale`. To change a resolution, drive the *inputs* the kernel resolves from and let it re-render:

- **(a) Record a per-slice authority override** to pin which source wins, then re-run `emery plan refine` — the amendment stales the slice's refinement manifest and the drain re-refines it.
- **(b) Amend the slice's sources** (drop a misleading source, or correct a source's authority class), then re-run `emery plan refine`.

Prose-only edits **outside** the kernel-rendered lines — clarifying the requirement body or a `Note:` line — are safe and never trip the validator.

## Resolve a divergence

A `[divergence]` already has an automatic winner (the higher-authority source); the slice is `refined` and buildable as-is.

1. Open `.emery/slices/<name>/specs/<domain>/spec.md` and read the inline commentary from the losing source.
2. If the authority winner is correct, accept it — optionally clarify the body prose (outside the kernel lines).
3. If the *wrong* source won, pin the source you want via a per-slice authority override or amend the plan's sources, then re-run `emery plan refine` so refinement re-resolves and re-renders the block:

```bash
emery plan amend <entry> --authority-override <kind>=<source>
```

Authority order: `intent` > `documentation` > `behaviour` (canonical: [Authority hierarchy](../../crates/slice/prompts/synthesis/authority.md)). The override surface key must already appear in the slice's `sources[]` — an orphan key is rejected by `emery slice validate` with `slice-authority-override-orphan-source`.

## Resolve a conflict

A `[conflict]` is a tie at the top authority class, so no source automatically wins. The slice still reaches `refined`; reconcile before executing — an undealt conflict is auto-deferred at the build gate (the requirement leaves build scope and is conserved as debt), and building *over* a `[conflict]` is never allowed.

1. Read both contributing Evidence files under `evidence/`.
2. Decide which source should win (or that a misleading source should be dropped), then drive the inputs:
   - Pin the winner with a per-slice authority override, **or**
   - Amend the plan to drop or re-bind the misleading source.

```bash
emery plan amend <entry> --authority-override <kind>=<source>
```

3. Re-run `emery plan refine` so refinement re-resolves the tie and re-renders `Sources:` / `Status:` and the headline tag.
4. Confirm the reconciliation landed:

```bash
emery slice validate <name>
```

Do **not** hand-edit `Status:` to `agreed` or strip the `[conflict]` tag — that desyncs the rendered block from `model.yaml` and fails `slice-spec-provenance-stale`.

## See also

- [Glossary](../appendices/glossary.md) — Conflict, Divergence, Authority
- [Skills](../reference/skills/index.md) — the `/emery:*` wrappers
- [Artifact format](../reference/artifact-format.md) — requirement block shape
