# Emery Target Architecture

> Status: **Draft v0.1** — spec-generator programme ([ADR-0008](decisions/0008-spec-generator-programme.md)). Written forward from the operator journey in [product.md](product.md), not backward from the current implementation. Phase 2 of [remediation-plan.md](remediation-plan.md) stamps this v1. Agent task briefs cite the live sections below, never the [deferred annex](#deferred-annex).
>
> The prior generation's failure mode was findings-driven repair with no destination ([architecture-review.md](architecture-review.md)). This document is the destination for *this* programme.

## 1. The journey, restated as architecture

```text
emery specify    sources → extract → per-source specs → synthesise → spec.md / design.md
                 (operator reviews; gaps and conflicts are inline [ADR-0004])
```

One lifecycle [ADR-0003]: legacy code, documentation, contracts, intent, screenshots, captures, and designs are *sources*. Architecture modelling (as-is, target, migration) is an optional projection over the same corpus, not a second product, and is not in this programme.

`emery build` / `status` / `fix` are destination verbs. They live in the deferred annex.

## 2. Shape

One process, one output home, one loop. **Wasm-primary deployment** (ADR-0002, accepted): the WIT component seam is the sole production seam — source adapters are Wasm components; first-party ones embedded in the binary as default registry entries; out-of-binary components admitted by one explicit install verb at an exact version. Adapter *logic* stays wasm-free behind `adapter::Source` (fast native unit tests); integration and grading cross the component seam with a scripted model backend. The native provider is deleted, not demoted.

The WIT for this programme is a source world: `extract` + `metadata`. No survey, no lead catalog, no target world.

### Kept kernels (ported, not rewritten)

| Kernel | Why it survives |
| --- | --- |
| The WIT component seam + Omnia hosting | Foundational (ADR-0002): dynamic adapter admission and desktop/web-service duality |
| Artifact parsers, collapsed to **one fail-closed spec AST** (addendum A17) | The typed serde parse as the load gate is sound; the two-parser split is not |
| Adapter source operations trait + prose corpus (extract prompts) | The trait is the seam; extract prose ports from `emery-adapters` at tag `v1` |
| Synthesis / authority prose | `intent` > `documentation` > `behaviour`; ports from `crates/slice/prompts/synthesis/` at tag `v1` |
| `error` / `diagnostics` | Unchanged leaves |
| Ultrathin skills (invoke-and-relay) | Honest; the CLI owns lifecycle |

Snapshot/CID identity ports only if sources must be pinned directory trees. The RFC-90 phase machine, target operations traits, refinement-as-separate-stage, and workspace kernel wait in the annex.

### Deleted planes (this programme)

The shadow native provider and its conversion layer, plus the five-mode adapter resolution matrix (ADR-0002); `crates/system` as a parallel lifecycle (ADR-0003); survey as a distinct operation, `lead` / `survey-result`, the target WIT world (ADR-0008); journal-as-authority reducers and the multi-writer claim protocol (ADR-0001); the guest HTTP mutating catch-all (addendum C3). In-place vs detached encodings are not rebuilt here (ADR-0005 Option C).

## 3. State model [ADR-0001]

Atomically swapped documents at the output home (ADR-0001 Option C): source bindings, extract receipts, the spec set. Properties:

- **One authority per question.** Bindings and receipts are documents, not directory-presence or mtime.
- **Fail closed.** Any authority read error is `Err`, never an empty set.
- **Resume is re-run `emery specify`.** A crash mid-write leaves the previous set: each run commits a fresh generation directory behind one atomically swapped `current` pointer, and readers trust only what the pointer names.
- **No journal, no journal-as-authority.** Observability is `wasi:otel` spans — emit-only from the guest, so nothing can read telemetry back for lifecycle; no log file exists in the output home.

A transactional store for waves, merges, and workspaces is a build-programme question.

## 4. Module map and budgets

Provisional layering. Budgets are ratchet ceilings, not goals; the archived engine at tag `v1` is ~101k lines against Omnia's ~30k for a whole runtime platform.

```text
error/diagnostics     — unchanged leaves
artifacts             — one spec AST (fail-closed); no engine deps; no leads catalog
adapter               — Source operations trait, seam DTOs (ONE claim family with extras, A8/A16), prose registry
engine                — the one loop: extract → synthesise → emit spec.md / design.md
transport             — clap grammar (`init`, `specify`), one RequestContext (C5), exit contract
host                  — engine-guest embedding, exact-pin component admission + embedded first-party
                        source registry [ADR-0002]
adapters (sibling)    — first-party source components; wasm-free cores, one export-macro guest
                        module each; target adapters frozen at tag v1
```

Ratchet ceilings (unconfirmed until v1): engine total well under the archived 101k — start from the skeleton and ratchet down, not up. CLI routes ≤ the live verb list (`init`, `specify`) plus an advanced namespace.

## 5. The adapter seam

- **The WIT is the source of truth; one generated type family** for the claim/spec records used everywhere the seam is parsed (A1, A8, A16). The claim record opens in the contract — core fields plus per-kind extras — so extract fails when required extras are absent. No "unmodeled keys are ignored" paths, and no hand-maintained mirrors of the WIT shapes.
- **Extract is the only source operation.** It takes a typed input (key, workspace-or-value) and returns specifications. Sources remain value-in: they do not read the output home, plan files, or lifecycle authority.
- Every adapter skip is a typed `not-applicable` or a blocking finding — no fail-open defaults (A14).
- Isolation profiles (D7) and dispatch budgets (D8) are seam properties, not prose — and they land with the build programme, not this skeleton (ADR-0008).

TargetContext, merge-on-the-phase-machine, and writable-artifact grants wait in the annex.

## 6. The reviewable specification

One spec set (`spec.md` / `design.md`) a human or reviewing agent can approve or reject without opening a second artifact family. It folds requirements, a provenance summary, open gaps, and conflicts. Structured models, if they exist, are subordinate. Reviewability is measured (time to first reviewable specification in T5 telemetry).

Conversational correction (`fix`) is destination surface — annex, not this programme.

## 7. Deliberately not built

Replaces the "there is no…" negative-space ledger. Each has a reopen trigger via ADR:

| Not built | Reopen when |
| --- | --- |
| Slice inventory, refine loop, collation | a Propellerhead spec that is wrong without them (ADR-0008 revisit) |
| Target WIT, phase machine, workspaces, merge | the build programme opens |
| Component *distribution* machinery — registries, pull-on-miss, bare-name resolution, marketplace | third-party adapter engagement (ADR-0002; the component seam and exact-pin install are foundational) |
| The web-service mutating ingress (auth, per-change anchoring, lease) | a hosted deployment is scheduled; until designed, mutating HTTP stays disabled (C3) |
| Second lifecycle / definition home | architecture-only engagement at scale (ADR-0003) |
| Capability profiles (D7) and dispatch budgets (D8) | the build programme, or a hosted ingress, is scheduled |
| Multi-node, streaming, hosted fleets, unattended merge | per [platform.md](platform.md) parked RFCs — measured pull only |

## 8. The walking skeleton

The executable definition of this document. Scripted model, offline, temp output home; **runs across the component seam** — the embedded engine guest plus a mock source component, so the shipped seam is the tested seam (ADR-0002, T1); runs in CI on every push; green is the definition of done for every remediation increment:

1. `emery specify` over an intent source (+ one docs source) → assert: synthesised `spec.md` / `design.md`, gaps typed `[unknown]`, conflicts visible inline [ADR-0004].
2. Re-run → byte-stable diff (empty).
3. Inject a source disagreement → `[conflict]` or `[divergence]` appears in the spec.

Build, status, fix, and crash-injection across merge boundaries are annex skeleton steps. They are not this programme's definition of done.

## Deferred annex

Conserved design intent for the **build programme**, re-derived after the generator ships — not current scope, not kept kernels, not Phase 3 acceptance. Retrieve the v0 text's build/status/fix detail from git history (`git show v1:rfcs/target-architecture.md` after this file's v0.1 lands, or the pre-trim draft). Headline only:

```text
emery build   reviewed spec → private workspace → phase machine → verified CID → baseline
              (resumable: re-run the verb; parallel later, serial until crash-proof)
emery status  one next action, from one state read
emery fix     durable, digest-bound guidance onto a stuck slice or wrong spec
```

Also conserved here, not built now: RFC-90 `build → verify ⇄ repair → review ⇄ repair`; merge on that machine (A9); `TargetContext` on every target operation (A10); waves as antichains (S32); publication as a drain-tail stage (S38); private workspaces (RFC-87); one transactional store if ADR-0001 Option A is accepted for *that* programme; D7/D8 isolation and budgets; snapshot/CID as delivery identity.

## See also

- [product.md](product.md) — the yardstick this document serves
- [capability-conservation.md](capability-conservation.md) — live generator capabilities vs deferred-with-build
- [remediation-plan.md](remediation-plan.md) — the path from tag `v1` to this document
- [architecture-review.md](architecture-review.md) — the evidence base; finding ids cited above resolve there
