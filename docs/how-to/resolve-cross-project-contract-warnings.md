# Resolve Cross-Project Compatibility Findings

After a producer contract changes, run `specify compatibility report --change <name>` or `specify compatibility check` to classify downstream consumer impact. Findings are advisory in RM-04, but `compatibility check` exits validation-failed for `breaking`, `ambiguous`, or `unverifiable` results so CI or operators can stop and triage.

This how-to covers where the findings appear, how to read them, and the four canonical responses.

## Prerequisites

- A multi-project change where at least one registry entry declares `contracts.produces` and another declares `contracts.consumes` against the same path.
- A producer contract in root `contracts/` and consumer workspace clones under `.specify/workspace/<consumer>/contracts/`.

For background on the check itself, see [Cross-project compatibility classification](cross-repo-contracts.md#cross-project-compatibility-classification-rm-04).

## 1. Find the findings

Run:

```bash
specify compatibility report --change <name>
```

Each finding includes the producer project, consumer project, producer contract, consumer view path, classification (`additive`, `breaking`, `ambiguous`, or `unverifiable`), optional `change-kind`, locator, and details.

## 2. Triage each finding

Four canonical paths:

### Path A: spawn a follow-up consumer change in the current plan

The consumer project needs to be updated to match the producer's new shape, and the work fits inside the current change. Add a new entry to `plan.yaml` that depends on the producer slice:

```bash
specify change plan add update-<consumer>-for-<producer-change> \
    --project <consumer> \
    --depends-on <producer-change> \
    --description "Adopt the updated <contract-path> contract from <producer-change>" \
    --context contracts/<contract-path>
```

Then re-run `/change:execute loop`. The new entry picks up on the next cycle once its dependency is `done`.

### Path B: spawn a follow-up consumer change

The producer change is shipping now and the consumer update is a separate beat (different release, different team, different review cycle). Land the current change as-is, then start a fresh change against the same hub:

```bash
# After landing the current change (specify change finalize)
specify change create adopt-<contract-path>-changes
# Edit change.md to point at the consumer projects
/change:plan adopt-<contract-path>-changes against ./
/change:execute loop
```

Record the compatibility report or PR discussion so the audit trail of "we knew about this drift when we merged the producer" is preserved.

### Path C: accept the drift (consumer is intentionally lagging)

Mobile shipping a release behind backend, or an external consumer outside your control, is a legitimate state. No code change is needed. Record the decision in the change, PR, or follow-up ticket:

```bash
# Example PR note:
# Compatibility finding on <consumer>/<contract-path> accepted:
# consumer ships in next release.
```

### Path D: producer change is wrong -- revert

The finding revealed a breaking change the producer should not have made. Revert the producer change on `main` (out-of-band -- Specify does not own the revert), then re-author the change against the consumer's expectations:

```bash
git revert <producer-merge-commit>
git push origin main
# Open a fresh change to redo the producer-side work
/spec:define <new-producer-change> description "Redo <producer-change> without breaking <consumer>/<contract-path>"
```

## 3. Verify the resolution

After applying any of paths A, B, or C, confirm the findings are accounted for:

```bash
specify compatibility report --change <name>     # findings are understood or now additive
specify slice status <consumer-change>           # if path A or B, follow-up change is tracked
specify change plan status                                # if path A, the entry is queued
```

Path D is verified by rerunning the producer work and then rerunning `specify compatibility check`.

## What the check does not do

- RM-04 does not transition plan state. `compatibility check` can fail for CI/operator attention, but RM-11 owns lifecycle gates.
- It never modifies consumer specs. The consumer's truth is in the consumer's specs and contracts -- the classifier only reports drift.
- It never auto-creates follow-up changes. The operator decides what to do.
- It does not run consumer builds or tests.

## See also

- [Cross-Repo Contracts](cross-repo-contracts.md) -- the broader contracts how-to.
- [Contract plugin](../reference/plugins/contract.md) -- the format-first skills and the separate compatibility CLI surface.
