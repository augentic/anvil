# Resolve Cross-Project Contract Warnings

After a producer change merges, `/change:execute` runs the cross-project compatibility check from RFC-9 Section 3B and surfaces any incompatibilities as warnings on the merge transcript and on the merged change's `journal.yaml`. The warnings are **advisory** -- they never halt the loop -- and the operator is on the hook to triage each one.

This how-to covers where the warnings appear, how to read them, and the four canonical responses.

## Prerequisites

- A multi-project initiative where at least one registry entry declares `contracts.produces` and another declares `contracts.consumes` against the same path.
- A producer change that has been merged via `/change:execute` (manually or via `--loop`).

For background on the check itself, see [Cross-project contract validation](cross-repo-contracts.md#cross-project-contract-validation-rfc-9-section-3b).

## 1. Find the warnings

Two surfaces carry the same data.

### On the merge transcript

`/change:execute`'s merge transcript prints a per-warning block right after the per-slice merge summary. Each block includes:

- The consumer project name.
- The contract path under inspection.
- The finding type (`schema-incompatible`, `binding-removed`, `enum-narrowed`, etc.).
- The validator's diagnostic detail.

Look for `cross-project-warning:` lines.

### On the merged change's journal

```bash
specify slice journal show <change>
```

Each warning is also written as a `cross-project-warning:` entry in the change's `journal.yaml` so the audit trail survives archiving. The journal is the canonical surface -- you can revisit it after the change is merged into the archive.

## 2. Triage each warning

Four canonical paths:

### Path A: spawn a follow-up consumer change in the current plan

The consumer project needs to be updated to match the producer's new shape, and the work fits inside the current initiative. Add a new entry to `plan.yaml` that depends on the producer change:

```bash
specify change plan add update-<consumer>-for-<producer-change> \
    --project <consumer> \
    --depends-on <producer-change> \
    --description "Adopt the updated <contract-path> contract from <producer-change>" \
    --context contracts/<contract-path>
```

Then re-run `/change:execute --loop`. The new entry picks up on the next cycle once its dependency is `done`.

### Path B: spawn a follow-up consumer change in a new initiative

The producer change is shipping now and the consumer update is a separate beat (different release, different team, different review cycle). Land the current initiative as-is, then start a fresh initiative against the same hub:

```bash
# After landing the current initiative (specify change finalize)
specify change create adopt-<contract-path>-changes
# Edit change.md to point at the consumer projects
/change:plan adopt-<contract-path>-changes --against ./
/change:execute --loop
```

The journal warning persists in the producer change's archive, so the audit trail of "we knew about this drift when we merged the producer" is preserved.

### Path C: accept the drift (consumer is intentionally lagging)

Mobile shipping a release behind backend, or an external consumer outside your control, is a legitimate state. No code change is needed -- the warning in the journal is the audit trail. Optionally, add a journal note to record the decision:

```bash
specify slice journal append <producer-change> \
    --phase merge \
    --kind recovery \
    --message "Cross-project warning on <consumer>/<contract-path> accepted: consumer ships in next release."
```

### Path D: producer change is wrong -- revert

The warning revealed a breaking change the producer should not have made. Revert the producer change on `main` (out-of-band -- Specify does not own the revert), then re-author the change against the consumer's expectations:

```bash
git revert <producer-merge-commit>
git push origin main
# Open a fresh change to redo the producer-side work
/spec:define <new-producer-change> --description "Redo <producer-change> without breaking <consumer>/<contract-path>"
```

## 3. Verify the resolution

After applying any of paths A, B, or C, confirm the warnings are accounted for:

```bash
specify slice journal show <producer-change>     # warnings still listed (audit trail)
specify slice status <consumer-change>           # if path A or B, follow-up change is tracked
specify change plan status                                # if path A, the entry is queued
```

Path D is verified by re-running `/change:execute --loop` against the redone producer change -- the cross-project check should now report zero findings.

## What the check does not do

- It never halts the executor. Warnings are best-effort, post-merge, advisory.
- It never modifies consumer specs. The consumer's truth is in the consumer's specs and contracts -- the validator only reports drift.
- It never auto-creates follow-up changes. The operator decides what to do.
- It does not run pre-merge. By design: pre-merge cross-project validation would couple the producer's merge to the consumer's clone state and re-introduce the federation problem RFC-9 Section 3B was carving out of.

## See also

- [Cross-Repo Contracts](cross-repo-contracts.md) -- the broader contracts how-to, including the full Section 3B description.
- [Cross-project contract warnings on the merge transcript](../appendices/troubleshooting.md#cross-project-contract-warnings-on-the-merge-transcript) -- troubleshooting entry.
- [`/change:execute` Cross-project contract check](../../plugins/change/skills/execute/SKILL.md) -- skill documentation for the executor's post-merge step.
- [Contract plugin](../reference/plugins/contract.md) -- the format-first skills (`/contract:openapi`, `/contract:asyncapi`, `/contract:json-schema`) whose verifier intent the executor invokes in `--mode cross-project`.
