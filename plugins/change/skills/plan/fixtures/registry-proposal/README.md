# Registry-proposal sub-step (RFC-9 §2B)

This fixture pins the **happy path** of the registry-proposal sub-step the plan skill runs in step 3(d).1 (`plugins/spec/skills/plan/SKILL.md` → §"Step 3(d).1 — Registry proposal sub-step").

## Scenario

`/change:plan platform-modernisation` is authoring a plan against a multi-project registry that already declares `legacy-monolith` and `command-centre`. The propose brief surfaces three slices; the assignment step routes two cleanly but flags `alpha-gateway-extract` as **unresolved** because no existing project owns the gateway responsibility surfaced in the discovery phase.

The operator types `alpha-gateway` as the project name. That name is not in `registry.yaml`, so the skill enters the registry-proposal sub-step:

1. Confirms with the operator (`Project alpha-gateway does not exist in registry.yaml. Create it now? [y/N]`).
2. Infers `--url` from the prefix shared by the existing entries (`git@github.com:augentic/`).
3. Defaults `--schema` to the majority schema (`omnia@v1`).
4. Prompts the operator for `--description` (required when the resulting registry is multi-project).
5. Shells out to `specify registry add alpha-gateway --url ... --schema ... --description "..."` and then `specify workspace sync`.
6. Returns to the assignment table, sets `alpha-gateway-extract.project = alpha-gateway`, and shells out `specify change plan amend alpha-gateway-extract --project alpha-gateway`.
7. Writes a `Registry amendments` block to `proposal.md` after the assignment table.

## Layout

| File | Pins |
|---|---|
| [`registry.yaml.before`](registry.yaml.before) | Pre-state — two existing projects, no `alpha-gateway`. |
| [`registry.yaml.after`](registry.yaml.after) | Post-state — `alpha-gateway` appended via `specify registry add`. |
| [`plan.yaml.before`](plan.yaml.before) | Plan with three pending entries; `alpha-gateway-extract` has no `project` (the assignment step hasn't routed it yet). |
| [`plan.yaml.after`](plan.yaml.after) | Same plan with all three entries routed via `specify change plan amend --project ...`. |
| [`discovery.md`](discovery.md) | The discovery brief's capability inventory that drove the propose pass. |
| [`proposal.md`](proposal.md) | Final proposal artefact, including the `## Assignment` table and the new `## Registry amendments` block (RFC-9 §2B). |
| [`transcript.md`](transcript.md) | The skill dialogue including the registry-proposal prompt, the inferred defaults, and the four shell-outs. |

## Key invariants

- **Verb order.** `specify registry add` precedes `specify workspace sync`; both precede `specify change plan amend --project <new>`. The plan validator rejects `project:` values not present in `registry.yaml`, so amending the plan first would be an `Error`-level finding.
- **`--description` required.** Multi-project registries enforce `description-missing-multi-repo` (RFC-3b). The skill never adds a project to a multi-project registry without a description.
- **Single-writer invariant.** The skill writes plan entries via `specify change plan add` only (in the propose step) and edits them via `specify change plan amend` only (in the assignment step). The registry-proposal sub-step adds the `specify registry add` write to the assignment step's responsibility — but never writes `plan.yaml` directly.
- **`--dry-run` suppresses every write.** Under `--dry-run`, none of `specify registry add`, `specify workspace sync`, or `specify change plan amend` runs. The output emits a `Would propose registry amendments:` block listing the same `(name, url, schema, description)` tuples.

## Counter-examples (not pinned)

- A `--dry-run` rendering of the same scenario.
- The decline path (operator answers `N` at the confirm prompt).
- A greenfield bootstrap (no `registry.yaml` at all, multi-project topology proposed by discovery). That flow is documented in the SKILL.md but is not fixture-pinned in this directory; see RFC-9 §2B for the contract.
