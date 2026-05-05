# Propose (step 3c)

Step 3(c) invokes the propose brief declared in `pipeline.plan` (for Omnia, `capabilities/omnia/briefs/plan/propose.md`; for Vectis, `capabilities/vectis/briefs/plan/propose.md`; other schemas ship their own). Propose reads `discovery.md`, applies the schema's slice heuristics to decompose the inventory into draft change slices with `depends-on` edges, and iterates with the human on each slice (accept / edit / reject / abort). For every accepted slice, the skill shells out to:

```text
specify plan add <name> \
    [--sources <key> ...] \
    [--depends-on <name> ...] \
    [--description "..."]
```

The `--description` carries scope and delta-targeting intent in prose; scoping is inferred from the description by the define skill at execution time.

Propose is the single-writer edge for plan entries — every entry lands via `specify plan add`; the skill never edits `plan.yaml` directly (see [SKILL.md](SKILL.md) §"Single-writer invariant"). The full decision trail (accepted, edited, rejected, skipped, aborted slices) is captured in `.specify/plans/<initiative-name>/proposal.md` regardless of per-slice decisions; the proposal header is exactly `# Proposal — <initiative-name>` with the same idempotency contract as `discovery.md`. The shape of a five-slice migration authoring run is pinned by `fixtures/propose/expected-plan.yaml` (final `plan.yaml`), `fixtures/propose/expected-proposal.md` (audit trail), `fixtures/propose/discovery.md` (step 3(a) inventory), and `fixtures/propose/transcript.md` (the interactive accept / edit / reject transcript). The per-slice prompt shape, the four legal actions (`y` / `edit` / `no` / `abort`), the edit sub-loop, and the rules governing dropped `depends-on` edges when a slice is rejected all live in the propose brief — see the schema's propose brief for the authoritative contract.

On abort, the skill writes `proposal.md` with the slices decided so far, skips step 4's validate (the plan is explicitly incomplete), and exits non-zero pointing the operator at `/spec:plan --extend` to resume. Partial plan entries from earlier accepted slices remain on disk — they were written synchronously by `specify plan add` and the skill never rolls those writes back. On a clean end-of-loop, step 4's `specify plan validate` is the final acceptance gate: any `Error`-level finding surfaces to the human with a recommended `specify plan amend` / `specify plan transition skipped` fix, never an in-skill edit.

## Context auto-population

When `/spec:plan` inserts changes, it automatically populates the `context` field on plan entries to help briefs focus on relevant baseline paths:

- **Contract changes**: When a contract change is inserted, implementation changes that depend on it get `context` entries for the contract paths the contract change will produce (e.g. `contracts/http/user-api.yaml`, `contracts/schemas/user.yaml`).
- **Spec changes**: When a change targets existing capabilities via `affects`, `context` entries are populated with the corresponding baseline spec paths (e.g. `specs/user-registration/spec.md`).
- **Manual authoring**: Operators can add context paths via `specify plan add --context <path>...` or `specify plan amend --context <path>...`.

Context paths are relative to `.specify/`. They are a focus hint — briefs may still read other baseline paths when instructed to.
