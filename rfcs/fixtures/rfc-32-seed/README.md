# RFC-32 seed fixture

Minimal consumer-project tree that proves the seeded deterministic hints
from RFC-32 Phase 2 step 6 produce non-empty findings. Wired up by Slice
S10 of [`rfc-32-plan.md`](../../rfc-32-plan.md).

## Layout

```
rfc-32-seed/
├── .specify/project.yaml      # minimal initialised project marker
├── crates/demo/src/lib.rs     # trips both seeded regex hints
└── README.md                  # this file
```

## Hints fired

| Rule       | Hint kind     | Match                                   |
| ---------- | ------------- | --------------------------------------- |
| `UNI-014`  | `regex`       | `https://example.com/api/v1/things`     |
| `OMNIA-002`| `regex`       | `std::env`                              |

Both rules scope to `**/*.rs` via a `kind: path-pattern` hint that runs
before the regex; the markdown and YAML files in the tree are excluded
from the candidate set.

## Reproducing the verification

From the `specify-cli` checkout:

```bash
cargo run --bin specrun -- \
  --format json \
  review run \
  --target omnia \
  --codex-root /Users/andrewweston/github.com/augentic/specify \
  --project-dir /Users/andrewweston/github.com/augentic/specify/rfcs/fixtures/rfc-32-seed \
  --output-format json
```

Expected outcome:

* exit code `2` (per RFC-32 §D8: any `important` / `critical` finding
  elevates the run).
* `summary.critical >= 1` (from `OMNIA-002`).
* `summary.important >= 1` (from `UNI-014`).
* Stdout is byte-identical across two consecutive invocations (§D9
  fingerprint stability).
