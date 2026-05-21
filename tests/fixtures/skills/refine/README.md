# `/spec:refine` synthesis fixtures

Worked-example fixtures for the synthesis playbook at [`plugins/spec/references/synthesis/`](../../../../plugins/spec/references/synthesis/) and the `/spec:refine` skill at [`plugins/spec/skills/refine/`](../../../../plugins/spec/skills/refine/).

Each fixture is one slice. The layout per fixture is:

```text
<fixture-name>/
  README.md          # what the fixture exercises + which playbook rules
  inputs/
    bindings.yaml    # the slice's resolved plan.yaml.slices[] entry plus its plan.yaml.sources map
    target.txt       # absolute repo-relative path to the target's shape brief
    evidence/        # one YAML per source-key (the post-extract input to synthesis)
      <source-key>.yaml
  expected/
    proposal.md
    spec.md
    design.md
    tasks.md
```

The Evidence YAMLs under `inputs/evidence/` validate against [`schemas/evidence.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/evidence.schema.json). The `expected/spec.md` requirement blocks validate against the W1.3 provenance parser at [`crates/domain/src/spec/provenance.rs`](https://github.com/augentic/specify-cli/blob/main/crates/domain/src/spec/provenance.rs).

## Fixture matrix

| Fixture                          | Sources                                | Tag expected             | Playbook rule exercised                                   |
| -------------------------------- | -------------------------------------- | ------------------------ | --------------------------------------------------------- |
| `single-source-intent/`          | `intent`                               | none (Status: agreed)    | Degenerate one-source path; intent drives proposal.       |
| `combined-docs-and-legacy/`      | `product-notes` + `legacy-monolith`    | none (Status: agreed)    | Combined evidence where sources agree per `claim-id`.     |
| `divergence/`                    | `identity-design-notes` + `legacy-monolith` | `[divergence]`      | `documentation > behaviour` resolves contradictory expiry values. |
| `conflict/`                      | `product-notes` + `identity-design-notes` (both documentation) | `[conflict]` | Tied top authority leaves the operator to reconcile.      |
| `unknown/`                       | `product-notes`                        | `[unknown]`              | Source emits `claims: []`; synthesis still surfaces the requirement gap. |

The five fixtures together cover RFC-25 §Acceptance scenarios `#5`, `#5a`, `#5b`, `#5c`, `#5h` (target `shape` injection happens in every fixture since all five target `omnia`).
