# `/change:execute` — `sources` / description-driven argument-wiring fixtures

These fixtures pin the three argument-shape variants `/change:execute` builds for `/spec:define` from a plan entry's `sources` and `description` fields. Delta targeting is description-driven: the define skill infers which baseline specs a change targets by reading the entry's description. The algorithm they illustrate lives in [`../../SKILL.md` → §Argument resolution](../../SKILL.md).

There is no automated harness that runs these fixtures. They are prose artefacts: a human reviewing a change to `/change:execute`'s argument-resolution code should be able to diff a new invocation rendering against the `invocation.txt` files here, and a change to the rendered transcript format against the `transcript.md` files.

## Layout

```text
field-wiring/
├── sources-only/
│   ├── plan.yaml          # one-entry plan, sources: [monolith], no description targeting
│   ├── invocation.txt     # pinned command line built from the entry
│   └── transcript.md      # rendered define step (extract sub-step present)
├── description-driven/
│   ├── plan.yaml          # one-entry plan, description targets user-registration, no sources
│   ├── invocation.txt     # pinned command line (no extra flags; delta inferred from description)
│   └── transcript.md      # rendered define step (no extract sub-step; delta targeting via description)
└── combined/
    ├── plan.yaml          # one-entry plan, sources + description-driven delta targeting
    ├── invocation.txt     # pinned command line (--source only; delta inferred from description)
    └── transcript.md      # rendered define step (extract sub-step + description-driven delta)
```

## Argument-shape matrix

| Fixture | `sources` | `description` (delta intent) | `/spec:define` extra flags |
|---|---|---|---|
| `sources-only/` | `[monolith]` | (no delta intent) | `--source monolith=/path/to/legacy` |
| `description-driven/` | (empty) | mentions `user-registration` | (none — delta inferred by define) |
| `combined/` | `[monolith]` | mentions `user-registration` | `--source monolith=/path/to/legacy` |

## Invariants every fixture asserts

1. **`--source` passes through the key verbatim.** The key (`monolith`) travels unchanged from the plan entry's `sources` list through the driver into the flag, so `/spec:define`'s brief pipeline can retain provenance when it hands the value to `/spec:extract`.
2. **`--source` value is the top-level `sources` map value, unchanged.** The driver neither stats local paths nor clones git URLs; it forwards the string as stored in `plan.yaml`. A missing path surfaces as a phase-level error from `/spec:extract`, not a driver-level one.
3. **Delta targeting is description-driven.** The `affects` field has been removed from the plan schema. The define skill reads the entry's `description` and infers which baseline specs the change targets. The driver does not emit any `--affects` flags.
4. **Greenfield invocation has no extra flags.** When a plan entry has no `sources` and no description referencing existing specs, the invocation is simply `/spec:define <name>` — no empty flag strings, no placeholder values. None of the three fixtures here demonstrate this case; the pre-existing greenfield fixtures under `../single-slice/success/` and `../loop/all-done/` cover it.
5. **Sources and description are independent.** The `combined/` fixture shows that a single entry can carry both `sources` and a description with delta intent; define handles the extract sub-step (from sources) and delta targeting (from description) independently. The driver does not coordinate between them.

## Using these fixtures

- Before changing the argument-resolution logic in `SKILL.md`, diff the `invocation.txt` files here against what the new algorithm would emit. Drift between the algorithm and these pins is a regression — update the fixtures in the same commit as the SKILL.md change.
- Before changing the rendered define-step output format, re-read the `transcript.md` files and confirm the new format still maps cleanly. The `Processing:` header suffix (`(sources: [...])` vs `(greenfield)`) is the load-bearing variant surface.
- End-to-end coverage of these signals across a multi-slice plan lives in `../e2e-platform-v2/`; that meta-fixture exercises the same argument-resolution code path on every entry of RFC-2 §"The Plan".
