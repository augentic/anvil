# Resolve spec conflicts

Handle `[conflict]` and `[divergence]` tags after refine synthesizes `spec.md`.

**Prerequisites:** A refined slice with tagged requirements; read [Core concepts](../explanation/concepts.md) § Evidence, provenance, authority.

## Understand the tags

| Tag | Meaning | Slice blocked? |
| --- | ------- | -------------- |
| `[divergence]` | Higher-authority source won; loser preserved as commentary | No — refine still reaches `refined` |
| `[conflict]` | Same-authority sources disagree; no automatic winner | No — operator reconciles |
| `[unknown]` | Insufficient evidence | No — mark for follow-up |

Tags appear on requirement headers alongside `Status: conflict`, `Status: divergence`, or `Status: unknown`.

## Resolve a divergence

1. Open `.specify/slices/<name>/specs/<unit>/spec.md` and read the inline commentary from the losing source.
2. If the authority winner is correct, accept as-is or edit prose for clarity.
3. If wrong, hand-edit the requirement or amend plan sources and re-run `/spec:refine`.

Authority order: `intent` > `documentation` > `behaviour`.

## Resolve a conflict

1. Read both contributing Evidence files under `evidence/`.
2. Edit `spec.md` to state the reconciled behavior explicitly.
3. Update `Status:` to `agreed` and remove the `[conflict]` tag when reconciled.
4. Run validation:

```bash
specify slice validate <name>
```

Optionally amend the plan to drop a misleading source before re-refining.

## Per-slice authority override

When you know which source should win for a claim kind:

```bash
specify plan amend <entry> --authority-override <entry> <kind>=<source>
```

Re-run refine after amending.

## See also

- [Glossary](../appendices/glossary.md) — Conflict, Divergence, Authority
- [/spec:refine](../reference/slice-skills/refine.md) — synthesis and validation
- [Artifact format](../reference/artifact-format.md) — requirement block shape
