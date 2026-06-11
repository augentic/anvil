# From sources to slices

Specify turns raw inputs — operator intent, written documentation, legacy code, screenshots, runtime captures — into behavioral specs. This page explains the two moments where that transformation happens, and how Specify keeps a clear trail back to where every requirement came from.

There are two distinct reconciliation moments, and they answer different questions:

- **Plan time — what work exists?** `/spec:plan` surveys each bound source for *leads* and reconciles them into the *slices* that make up the change.
- **Slice time — what must each domain do?** `/spec:refine` extracts *evidence* from each source and synthesizes it into the domain's `specs/<domain>/spec.md`, recording exactly which source contributed each requirement.

## Plan time: leads become slices

### A lead is a unit of work a source can see

When you run `/spec:plan`, each bound source runs its `survey` operation and emits **leads** — one block per slice-sized unit of work it can identify, written under `## Lead inventory` in `discovery.md`. A lead is identified by its `(source, lead)` pair, because the same lead name can appear in more than one source.

For example, a legacy-code source and a design-notes source might each surface a `user-registration` lead. They are describing the same feature, but `survey` does not yet know that — it only reports what each source sees on its own.

### Propose reconciles leads across sources

The cross-source matching happens in the **propose** sub-step of `/spec:plan`. The agent reads every lead, judges which ones describe the same piece of work, and emits the `slices[]` rows directly — each row naming its matched leads, *at most one per source*.

Three rules keep this predictable:

- **One lead per source, per slice.** A slice never fuses two leads from the *same* source. Re-sizing same-source work is an operator action at Gate 1, not something the agent does silently.
- **Coverage is at-least-once, not exactly-once.** Every surveyed lead must be referenced by at least one slice, and a lead may appear in more than one. Work that lands in more than one project becomes multiple slices joined by `depends-on`; a cross-cutting lead — say a conventions document that informs several features surfaced by another source — is bound into every slice it informs (the one-lead-per-source rule still applies inside each slice), with no `depends-on` implied. Multi-homed leads are listed in `change.md` under `## Cross-cutting leads` for Gate 1 review.
- **Uncertain matches are surfaced, not hidden.** When the agent is unsure whether two leads are the same work, it records the pair under `## Tentative merges` in `change.md` so you can confirm or split them. When two matched leads materially disagree, the slice is flagged `divergence: likely`.

This is why a one-source, one-lead change and a twelve-slice migration use exactly the same machinery — the only difference is how many leads `survey` produced.

You review and adjust the proposed slices at **Gate 1** before stamping the plan `approved`.

## Slice time: evidence becomes a spec

### Extract gathers evidence per source

When `/spec:refine` runs for a slice, each bound source runs its `extract` operation against its matched lead and returns an **Evidence** document, persisted to `.specify/slices/<slice>/evidence/<source>.yaml`. Evidence is structured: a list of `claims` (requirements, criteria, decisions, code excerpts, and so on) plus a top-level `authority` that records how much weight the source carries.

### Synthesize reconciles evidence into one spec

The slice then runs **synthesize**, which reconciles every source's Evidence into a single set of requirements. Two artifacts come out of this step:

- `specs/<domain>/spec.md` — the human-readable behavioral spec, one file per domain.
- `model.yaml` — a structured, machine-readable record of the same requirements, carrying provenance inline.

Each requirement in the spec carries three provenance lines:

```markdown
ID: REQ-001
Sources: [identity-design-notes, legacy-monolith]
Status: agreed
```

- **`ID:`** is a stable `REQ-XXX` identifier — the merge key that survives renames and lets later slices modify this requirement precisely.
- **`Sources:`** is the **provenance**: which sources contributed the requirement, highest authority first.
- **`Status:`** is a closed enum — `agreed`, `unknown`, `conflict`, or `divergence`.

## How disagreements are resolved

Two sources can disagree about the same requirement. Specify resolves this with **authority** — a closed ranking declared per source (`intent` > `documentation` > `behaviour`), sharpened by an optional per-slice override the operator records at Gate 1. The winner's value becomes the operative requirement and the loser survives as inline commentary (`[divergence]`); a tie at the top authority class has no winner (`[conflict]`). The canonical hierarchy, override surface, and step-by-step resolution order live in [Authority hierarchy](./synthesis/authority.md#resolution-order).

Tags never park the slice. Synthesis tags the requirement and proceeds. The operator reconciles a `[conflict]` or `[divergence]` by recording a per-slice authority override (`specify plan amend --authority-override`) or amending the plan's sources, then re-running `/spec:refine` — never by hand-editing the kernel-rendered `spec.md` provenance lines.

## model.yaml and the provenance trail

`model.yaml` (at `.specify/slices/<slice>/model.yaml`) is the single structured record of a refined slice. It holds the requirement set with **inline provenance** — for each requirement, which claims contributed and which one won — plus the task list and a small header. It is the artifact `specify slice validate` checks for drift, and it is what later steps read instead of re-parsing the markdown.

There is no separate `provenance.yaml` on disk. The full audit view is *projected on demand* from `model.yaml` and the Evidence files by `specify slice provenance`, so the trail can never drift out of sync with the spec.

## See also

- [Authority precedence](./synthesis/authority.md) — resolution order for skill authors
- [Anatomy of an adapter](https://specify.augentic.io/explanation/adapter-anatomy.html) — how sources emit leads and evidence
