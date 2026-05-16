# Propose (step 4c)

Step 4(c) invokes the propose brief bundled with this skill at `briefs/<capability>/propose.md` (for Omnia, [`briefs/omnia/propose.md`](briefs/omnia/propose.md); for Vectis, [`briefs/vectis/propose.md`](briefs/vectis/propose.md); other capabilities ship their own variant alongside). Planning is orchestration, not capability-owned slice work, so planning briefs live with this skill rather than in `capability.yaml`. Propose reads `discovery.md`, applies the capability's slice heuristics to decompose the inventory into draft slices with `depends-on` edges, and iterates with the human on each slice (accept / edit / reject / abort). For every accepted slice, the skill shells out to:

```text
specify plan add <name> \
    [--sources <key> ...] \
    [--depends-on <name> ...] \
    [--description "..."]
```

The `--description` carries scope and delta-targeting intent in prose; scoping is inferred from the description by the define skill at execution time.

Propose is the single-writer edge for plan entries: every entry lands via `specify plan add`, and the skill never edits `plan.yaml` directly (see [plan-single-writer.md](../../references/plan-single-writer.md)). The full decision trail (accepted, edited, rejected, skipped, aborted slices) is captured in `.specify/plans/<change-name>/proposal.md` regardless of per-slice decisions; the proposal header is exactly `# Proposal — <change-name>` with the same idempotency contract as `discovery.md`. The shape of a five-slice migration authoring run is pinned by `fixtures/propose/expected-plan.yaml` (final `plan.yaml`), `fixtures/propose/expected-proposal.md` (audit trail), `fixtures/propose/discovery.md` (step 4(a) inventory), and `fixtures/propose/transcript.md` (the interactive accept / edit / reject transcript). The per-slice prompt shape, the four legal actions (`y` / `edit` / `no` / `abort`), the edit sub-loop, and the rules governing dropped `depends-on` edges when a slice is rejected all live in the propose brief — see the capability's propose brief for the authoritative contract.

On abort, the skill writes `proposal.md` with the slices decided so far, skips step 5's validate (the plan is explicitly incomplete), and exits non-zero pointing the operator at `/change:draft <name> extend` to resume. Partial plan entries from earlier accepted slices remain on disk — they were written synchronously by `specify plan add` and the skill never rolls those writes back. On a clean end-of-loop, step 5's `specify plan validate` is the final acceptance gate: any `Error`-level finding surfaces to the human with a recommended `specify plan amend` / `specify plan transition skipped` fix, never an in-skill edit.

## Context auto-population

When `/change:draft` inserts entries, it automatically populates the `context` field on plan entries to help briefs focus on relevant baseline paths:

- **Contract slices**: When a contract slice is inserted, implementation slices that depend on it get `context` entries for the contract paths the contract slice will produce (e.g. `contracts/http/user-api.yaml`, `contracts/schemas/user.yaml`).
- **Spec slices**: When a slice targets existing capabilities via `affects`, `context` entries are populated with the corresponding baseline spec paths (e.g. `specs/user-registration/spec.md`).
- **Manual authoring**: Operators can add context paths via `specify plan add --context <path>...` or `specify plan amend --context <path>...`.

Context paths are relative to `.specify/`. They are a focus hint — briefs may still read other baseline paths when instructed to.
