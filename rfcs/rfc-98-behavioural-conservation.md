# RFC-98: Behavioural Conservation

> Status: Draft — conservation-oracle track following [RFC-97](rfc-97-native-verification.md)
>
> Owns: the conservation corpus as a protected oracle, the `conserve` verification profile, replay verdicts and their normalization, declared-divergence reconciliation against the slice requirement set, per-requirement conservation coverage, and the conservation floor as an execution-policy input.
>
> Depends on [RFC-97](rfc-97-native-verification.md) for host execution, protected oracles, and assurance, and on the `captures` source adapter for the recorded corpus. Benefits from [RFC-94](rfc-94-target-readiness.md) `behavioural-observability`.
>
> Patch ownership: this RFC extends RFC-97 D2's closed profile-name set with `conserve` and RFC-97 D4's protected-oracle contract with corpus admission. RFC-97 remains unchanged.
>
> Evidence posture: [platform evaluation](platform.md#evidence-and-iteration-posture).

## Intent

*Prove that the migrated system still does what the old one did, using the same evidence that told us what the old one did.*

Emery recovers a specification from a running legacy system. The `captures` source adapter consumes runtime capture trees and emits `kind: example` Evidence claims anchored by `replay-digest: sha256:…`. That evidence flows inward into synthesis and then stops.

At build time the strongest available assurance is RFC-97's `test` profile over candidate-authored tests, which reports `assurance: candidate`, plus a model review. For a modernization engagement that is the wrong question answered well. "Does the new code pass the tests written alongside it" is weak evidence, because the same process produced both. The load-bearing question is whether the new system reproduces the recorded behaviour of the old one.

The capture corpus is the only input in the change that is neither model-authored nor model-writable: it came off the running legacy system before any slice existed. That makes it the natural protected oracle, and this RFC turns it into one.

## Problem

Three gaps sit between today's evidence and a defensible conservation claim.

**Captures are single-direction.** A `replay-digest` anchor identifies a recorded interaction, but nothing replays it. The digest proves the claim came from a real observation; it never proves the observation still holds.

**Candidate assurance is the ceiling.** RFC-97 D4 draws the right distinction and then leaves most legacy targets on the wrong side of it. `protected-oracles[]` exists, but a brownfield estate rarely has a pre-existing trustworthy test suite to admit into it. The corpus is the oracle that estate actually has.

**Conservation is not projected.** There is no artifact that answers, per requirement, whether the delivered behaviour was verified against a recording or only reviewed. For fixed-price delivery underwritten by the audit trail, that projection is the deliverable, not a by-product.

## Terms

- A **conservation corpus** is the digest-bound set of recorded interactions the `captures` adapter surveyed and extracted for a change, retained by `replay-digest` and immutable for the change's life.
- A **replay** is one deterministic re-execution of one recorded interaction against a candidate, under a deployment-owned driver.
- A **conservation verdict** is the closed value `conserved | diverged | declared | inapplicable | uncovered`.
- A **conservation report** is the host's normalized result for one replay set against one candidate, on the RFC-97 profile-report shape.
- **Conservation coverage** is the projection from a slice's requirements to the replays that back them.
- A **conservation floor** is the minimum coverage an execution policy admits.

## Decisions

### D1 — The corpus is a protected oracle, not a test suite

The conservation corpus is admitted through RFC-97 D4's `protected-oracles[]` by digest and mounted read-only outside the candidate tree. It is never materialized into the workspace, never included in snapshot capture, and never within any task owner's ownership envelope.

This is the whole mechanism. A slice that could resolve a failing replay by editing the recording would convert the one trustworthy input in the change into another model-writable artifact, and the resulting attestation would mean nothing. Replay evidence earns `assurance: protected` by construction, and only because the corpus is unreachable from every writer.

Corpus admission is covered by member admission on the same rule as every other protected input, so the exact recording set that a build was verified against is bound into the epoch and reproducible from the fact log.

### D2 — `conserve` is a verification profile, not a new phase

`conserve` joins RFC-97 D2's closed profile names. Nothing else about the phase machine changes: target metadata declares it in its ordered required set per platform, the adapter requests it by name, the host selects the pre-bound policy, executes, normalizes, stores, and returns an opaque attestation handle that the engine resolves directly.

Deployment policy owns the replay driver per target and platform, exactly as it owns the command mapping for `build` or `test`. A transcript-shaped corpus replays through a request driver; a terminal corpus replays through a terminal driver; an interaction corpus replays through a UI driver. Adapters supply no driver, no command, no comparison rule, and no tolerance.

Normalization follows RFC-97 D5. Volatile content that does not identify a behavioural difference — timestamps, generated identifiers, ordering that the recording did not constrain, host and path roots — is removed by the profile policy before comparison, so a replay set produces byte-identical reports and fingerprints across runs. The tolerance rules are policy data with a digest, not a model judgment, because a tolerance that a model can widen is not an oracle.

### D3 — Divergence is reconciled against the requirement set, not treated as failure

A migration that conserves everything has delivered nothing. Conservation cannot demand bug-for-bug fidelity, so a raw divergence is not automatically blocking.

Each replay verdict is reconciled against the slice's requirement set before it enters the phase report:

| Verdict | Meaning | Blocking |
| -------------- | ------------------------------------------------------------------------------------ | -------- |
| `conserved` | The candidate reproduced the recording within the profile policy's tolerance | No |
| `declared` | The candidate diverged, and a requirement carrying `[divergence]` names this behaviour | No |
| `diverged` | The candidate diverged with no requirement declaring the change | Yes |
| `inapplicable` | The recording exercises behaviour the change explicitly removed, per a requirement | No |
| `uncovered` | No recording reaches the requirement | No, counts against coverage |

Reconciliation reuses the existing `[divergence]` tag on `spec.md` requirement headers and the authority resolution behind it. The specification is where intended change is declared, so the specification is what licenses a divergence. Nothing else can: not the adapter, not the review leg, and not the model that wrote the code. An undeclared divergence is a blocking finding routed through RFC-90's ordinary repair budget, and its resolution is either a code fix or an operator amending the specification to declare the change — both of which leave a record.

This preserves the authority ordering rather than bending it. `captures` Evidence carries `authority: behaviour`, the lowest tier, and that is still correct for deciding what the system *should* do. Conservation does not promote it. The corpus is authoritative only about what the old system *did*, which is a different claim, and the specification remains authoritative over both.

### D4 — Conservation coverage is a projected audit artifact

Coverage is projected, never stored, on the same rule as every other Emery status:

```text
emery slice conservation <slice>
```

For each requirement in `model.yaml`, the projection resolves the replays whose recordings contributed to its provenance and reports the strongest verdict backing it. The change-level roll-up is the artifact an engagement actually hands over: how many delivered requirements are backed by a replayed recording, how many by candidate checks alone, how many are uncovered, and which divergences were declared and by whom.

The roll-up is snapshotted onto `target.merge.wave-committed` and summarized by `plan archive`, so the conservation position of an accepted CID is recoverable from the fact log after the change home is deleted.

### D5 — Conservation gates by policy, and an absent corpus is never a silent pass

A conservation floor is an execution-policy input resolved on the established precedence — `--conservation-floor` flag, then a `project.yaml` declaration, then the default of zero — and recorded in `plan.execute.started` coverage beside the gap policy.

The floor is checked at the same gate as gaps, before build. Two rules keep it honest:

- A floor above zero with no admitted corpus fails typed `conservation-corpus-absent`. It never passes vacuously, because a target with nothing to replay would otherwise report perfect conservation.
- A floor above zero requires the target to declare `conserve` in its required profile set. A floor the phase machine cannot evaluate is a configuration error, not a satisfied constraint.

The default of zero keeps every existing change working unchanged. Greenfield work has no prior behaviour to conserve and correctly carries no floor.

### D6 — Conservation telemetry is raw evidence

Each replay set emits RFC-97 D9's `target.verify.profile-completed` for the `conserve` profile with no new event kind, plus per-verdict counts. RFC-93 outcome records carry the change's terminal coverage roll-up, so recurrence analysis can ask which corpus shapes actually predicted post-merge defects. Telemetry is observation; no count changes a lifecycle transition.

## Implementation requirements

- Extend the `captures` source contract so extraction retains the recorded interactions under their `replay-digest` as a corpus value with a canonical set digest, rather than emitting anchors alone.
- Add corpus admission to RFC-97 D4 `protected-oracles[]` handling, including member-admission coverage of the exact corpus digest.
- Add `conserve` to the closed profile-name set, the target metadata required-profile vocabulary, and the deployment profile-policy registry, with the first-party request, terminal, and interaction drivers and their tolerance policies.
- Add the closed `ConservationVerdict` enum, reconciliation against `spec.md` `[divergence]` tags and `model.yaml` provenance, and routing of `diverged` into RFC-90's existing blocking-finding path with no new repair budget.
- Add read-only `emery slice conservation` and the change-level roll-up; snapshot the roll-up on wave commit and summarize it in `plan archive`.
- Add the conservation floor to execution-policy resolution and the pre-build gate, with `conservation-corpus-absent` and the missing-profile refusal.
- Extend RFC-93 outcome records with terminal conservation coverage.
- Integration coverage for corpus write-denial from every task owner, replay normalization determinism, declared-versus-undeclared divergence routing, vacuous-pass refusal, and coverage projection against a scripted driver.

## Acceptance criteria

1. A corpus admitted as a protected oracle is unreachable from every task owner's ownership envelope; an attempt to write it fails the attempt rather than passing a replay.
2. Replaying the same corpus against the same candidate twice produces byte-identical normalized reports and fingerprints, with volatile content removed by policy.
3. A conserved replay set reports `assurance: protected` through direct engine handle resolution and `phase-source: tool`.
4. A candidate that changes a recorded behaviour with no declaring requirement produces a blocking `diverged` finding routed through RFC-90's existing repair budget. Adding a `[divergence]` requirement that names the behaviour reclassifies it as `declared` on the next attempt.
5. A conservation floor above zero with no admitted corpus fails typed `conservation-corpus-absent` before build, and a floor above zero against a target that does not declare `conserve` is refused as configuration.
6. A change with no floor and no corpus behaves exactly as it does today.
7. `emery slice conservation` projects per-requirement verdicts from `model.yaml` provenance, and the change roll-up survives archive on the wave-commit fact.
8. Outcome records carry terminal conservation coverage, and removing a change from the analysed set changes the analysis digest.

## Rejected alternatives

- **Replay recordings as ordinary candidate tests.** Materializing the corpus into the workspace makes it writable by the process being verified and collapses the `candidate` / `protected` distinction that gives the result its meaning.
- **Require bug-for-bug conservation.** A migration exists to change something. Without declared divergence the gate would block every valuable change and be switched off, which is worse than not having it.
- **Let the review leg decide whether a divergence was intended.** That is the specification's job. Asking the model that wrote the code whether the behaviour change was deliberate is not an independent check.
- **Promote `captures` Evidence above `documentation` or `intent` authority when a replay conserves.** Conservation says what the old system did, not what the new one should do. Authority ordering is unchanged.
- **Generate the corpus from the new system when the legacy recording is thin.** Circular: the candidate would define its own oracle.
- **A separate conservation phase in the build machine.** Conservation is a check over a candidate, which is exactly what a verification profile is. A new phase would duplicate budgets, reports, and gates for no additional expressiveness.
- **Store coverage on `model.yaml` or `plan.yaml`.** Coverage is computed from facts and the corpus, on the same rule that keeps every other status projected.
