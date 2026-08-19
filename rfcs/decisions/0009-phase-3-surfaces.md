# ADR-0009: Phase 3 surfaces — source bindings, output home, required extras, journey host

> Status: **Accepted** (remediation Phase 3, 2026-08-19). The walking-skeleton cut of
> [remediation-plan.md](../remediation-plan.md) Phase 3, implementing
> [target-architecture.md](../target-architecture.md) §§2–6, 8 under ADR-0001/0004/0005/0008.
> Date: 2026-08-19

## Context

Phase 3 builds the spec-generator spine: output home → extract → synthesise → emit
`spec.md` / `design.md`, green across the component seam. The standing ADRs fix the
direction (ADR-0008: two verbs, first artifacts; ADR-0001 Option C: generation-pointer
state; ADR-0004 Option D: inline conflict; ADR-0005 Option C: one output home) but four
surface details are still open, and each touches an ADR-gated path (`wit/`,
`crates/transport/src/command/`, `crates/artifacts/src/`). This record fixes them so the
skeleton lands against decisions, not improvisation.

## Decision

### 1. Source bindings are declared at `init`

`emery init` grows from one adapter to a source-binding list, still two verbs, no new
operator nouns beyond ADR-0008's *source*:

- `emery init <adapter>...` — each positional resolves one source adapter (local
  component path, exact pin, or bare name; ADR-0007 cache-seed precedence conserved) and
  records one workspace-backed binding whose key is the adapter name and whose read-only
  view root is the project directory.
- `emery init --value <adapter>=<text>` (repeatable) — records one value-backed binding:
  the adapter extracts the inline text (the WIT `content.value` arm). This is how an
  intent source binds in the skeleton.
- `--upgrade` remains the re-entry path (re-ensure recorded bindings, bump the pin).

Bindings persist on `.emery/project.yaml` under a `sources:` list (`key`, `adapter`,
`workspace` or `value`). The v1 single `adapter:` key, the `rules:` scaffold map, and
`--platforms` are deleted with the target axis — they are build-programme vocabulary
(pre-1.0 hard cut; re-init, no migration). Init resolves the **source axis only**; the
target axis stays deleted (`adapter-axis-removed` is unreachable from the live grammar).

### 2. Output-home layout: content-addressed generations behind one pointer

ADR-0001 Option C, made concrete:

```text
.emery/
  project.yaml               # name, emery pin, sources (the authored bindings)
  spec/
    current                  # the generation pointer: one line naming a generation id
    generations/<id>/        # one complete, immutable spec set
      bindings.yaml          # resolved bindings snapshot for this run
      receipts.yaml          # one extract receipt per source (identity + claim digest)
      spec.md
      design.md
```

The generation id is the SHA-256 digest of the set's canonical bytes, so an identical
re-run converges on the same directory and the pointer swap is a no-op — byte-stable
re-run is structural, not incidental. Commit order: write the full generation directory,
`sync`, then atomically replace `current` (the temp-write / `sync_all` / rename envelope
in `crates/artifacts`); after a successful swap, generations not named by the pointer are
pruned. Readers trust only what the pointer names and fail closed on everything else. No
document in the output home carries a timestamp or a log line.

### 3. Required extras are a closed engine-side table, fail-closed

The WIT claim record stays open (`extras: list<tuple<string, string>>`, A8); enforcement
is the engine's load gate (A8/A16 as contract, not prose). Per claim kind:

| Kind | Required extras | Required core fields |
| --- | --- | --- |
| `requirement` | `statement` | `id` |
| `criterion` | `criterion` | `id` |
| `example` | `replay-digest` | `id` |
| every other kind | none | — |

A claim missing a required entry fails extract with a typed error naming the source, the
claim, and the missing key — never a `synopsis` fallback, never "unmodeled keys are
ignored". Widening the table is a contract change gated by this file's path rule.

### 4. Synthesis is model-in-the-loop; honesty stays mechanical

The engine computes a deterministic reconciliation over the typed claims first — group by
claim id, resolve what authority precedence (`intent` > `documentation` > `behaviour`)
can resolve as `[divergence]`, record same-authority disagreement as `[conflict]`, and
requirements with no acceptance criterion as `[unknown]` gaps. The ported v1 synthesis
prose (from `v1:crates/slice/prompts/synthesis/`; the review-time port keeps five of the
eight prompts and drops `boundary`, `decisions`, and `substeps` as build-programme
vocabulary) plus the claims and that reconciliation
go to the model (`wasi:model`); the answer is the `spec.md` / `design.md` text. The
engine then fails closed unless the answer parses under the one spec AST (A17) **and**
carries every reconciliation tag inline (ADR-0004 Option D) — a model answer that hides a
conflict is a typed error, not a spec.

### 5. The journey host: a scripted model behind the same seam, never a shipped flag

The shipped binary keeps `WasiModel: Cursor` with no lab flags (operator decision,
2026-08-19). The journey drives a dev-only host harness (`crates/journey-host`,
`publish = false`) embedding the **same guest bytes** with the same mounts and resolver
wiring; only the model host capability differs — omnia-testkit's `Scripted` (which
implements the host-side `WasiModelCtx`) answering from a script file the test names.
ADR-0002 §1 already records the model as a host capability whose scripting requires no
guest change; the component seam under test is byte-identical to the shipped one (CC-17).

### 6. Mechanical enforcement

New ratchet entries (`engine`, `journey-host`) cite this ADR; the T9 spine cut re-homes
the surviving handler/resolver kernels into `engine` (its ceiling absorbs the port, and
every other crate ratchets down); `tests/layering.rs` gains the new-spine edges (and is
revised again at the Phase 3 spine cut). The journey's
`adr_0001_*` / `adr_0004_*` tripwires stand; crash injection (a kill between generation
write and pointer swap leaves the previous set) joins the journey per ADR-0001's spike
clause.

## Deletions

`--platforms` and the `platforms:` config key from the live grammar; the `rules:`
scaffold map and its `proposal` / `specs` / `design` / `tasks` keys; the single
`project.yaml.adapter` binding key (replaced by `sources:`); target-axis resolution from
the live init path; `specify-not-implemented` (the stub becomes the generator). Net
concept-count effect: the operator model gains one noun detail (a *binding* on the
existing *source* noun) and loses three (`platforms`, `rules`, the axis distinction).

## Consequences

Existing v1-shaped projects do not load (pre-1.0 hard cut; `emery init` re-scaffolds).
The interim native rung gains extract dispatch on the compiled catalog so per-push tests
stay fast until the Phase 3 spine cut deletes the native provider (ADR-0002); the
component seam remains the only production seam throughout.

## Revisit trigger

Phase 4's first real source port (documentation / code / intent prose from
`emery-adapters` at tag `v1`): if the required-extras table or the binding surface cannot
express a first-party adapter's contract, this record is amended by a successor ADR — not
by loosening the load gate in place.
