# RFC-97 Phase A implementation plan

> Status: Ready to staff — sequential, session-sized steps for [RFC-97](rfc-97-native-verification.md) Phase A (`slice-attempt` host verification). Not a new RFC.
>
> Phase B (`frontier-domain | complete-domain`, RFC-96 D8 closure) is out of scope. Do not start a step until every preceding step is complete. The implementing agent does not create commits, branches, or pull requests — git stays with the operator. One branch per repository carries every step; the operator opens one pull request per repository when the plan is done.

## How to use this plan

Each step is one agent session. The agent implementing a step:

1. Reads this document's preamble, the named step, and the cited RFC-97 sections. Does not re-derive Phase B or parked RFCs.
2. Implements only that step's work. Leaves later steps untouched.
3. Compiles the affected crates and runs the tests named in the step. Does not run `cargo make ci` unless the step says so.
4. Stops. Does not commit, amend, rebase, push, or open a pull request.
5. Reports what changed, which tests ran, and anything that blocked the step.

The operator reviews, commits, and (after the last step in each repository) opens the pull request.

## Repositories

| Repository | Role in Phase A | Pull request |
| --- | --- | --- |
| [`augentic/emery`](https://github.com/augentic/emery) | Engine, WIT, capability crate, seam, mock verifier, orchestration, docs | One PR covering steps 1–15, 18–19 |
| [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters) | Omnia declares and requests host profiles; contracts and vectis stay on today's `report` path | One PR covering steps 16–17 |

Co-development: uncomment the path patches in `emery-adapters/Cargo.toml` so the adapters branch resolves `emery-adapter` / `emery-native` from the sibling `emery` checkout. Publishing `emery:adapter` / `emery:verification` and cutting a release are operator release acts, not plan steps.

## Scope

Phase A delivers host-attested `slice-attempt` verification on implemented RFC-90. It does **not** open domain context variants, domain repair, domain cache reuse, RFC-100 publication, or RFC-106 task grants.

Every Phase A implementation-requirement bullet and acceptance criterion is assigned to a step in [Coverage](#coverage).

## Plan decisions

These resolve shape that RFC-97 leaves to the implementer. They are not open questions. Prefer them over inventing a richer surface.

1. **Required-profile metadata.** Add `required-profiles: list<profile-set>` to target `adapter-metadata`. A `profile-set` is `{ platform: option<platform>, profiles: list<string> }`. An empty list means the target declares no host profiles and keeps `verify-result::report`. `platform: none` means the set applies to `core` (and to a platform-agnostic target). `ci` remains mutually exclusive with every other name **inside one set**.
2. **First-party Omnia required set.** Omnia declares `fmt`, `build`, `clippy`, `test` for `core`. The deployment registry may know all eight closed names; Omnia metadata does not require `doc`, `vet`, `deny`, or `ci` in Phase A.
3. **Protected-input wire names.** Keep the reserved Node fields `covered` and `oracles`. Do not rename them to the RFC's conceptual `protected-verification-inputs[]` / `protected-oracles[]`.
4. **Protected-input writers.** Target metadata may nominate optional defaults (`default-covered`, `default-oracles`). `emery plan author` copies those defaults onto a leaf Node that has none. `emery plan amend` is the operator override (`--covered` / `--oracle` / clear flags). No other verb writes the fields. Defaults are inert until that decomposition revision is admission-covered.
5. **Adapter-facing WIT vs engine bind.** Package `emery:verification@0.1.0` has two interfaces. `verifier` (`run-profile`, `resolve`) is imported by the target world and the workflow world — no argv, policy, target, or protected-input selector. `control` (`bind`, `preflight`) is imported only by the workflow world. The engine pre-binds context against the candidate workspace id before `target.verify`; the adapter names only a profile and a workspace handle.
6. **Multi-platform multiplexing.** The bound context carries an ordered list of `(platform, required profiles)`. Each `run-profile(name)` fills the first unbound slot for that name. Extra calls for a filled slot are `verification-attestation-duplicate`. Phase A Omnia is `core` only; a host-profile target on a multi-platform project that the registry cannot serve fails preflight as `verification-platform-unsupported`.
7. **Attestation handle.** Opaque string: `sha256:` hex digest of the persisted normalized report bytes. Persist at `build/attempts/<attempt>/attestations/<handle>`.
8. **Lineage cache root.** `$EMERY_HOME/verification-cache/<lineage-key>/` (else `$HOME/.emery/verification-cache/…`). The key is the D8 tuple. Cache contents never enter snapshot capture.
9. **Portable sandbox floor.** Phase A enforces and attests `workdir-bind`, `env-allowlist`, `no-inherited-credentials`, `resource-limits` (wall time, stdio bytes, process count), `process-tree-reap`, `ephemeral-write-roots`, and `protected-input-readonly`. `egress-deny` is recorded when the host can actually enforce it; the first-party Omnia policy does not demand an unenforceable feature. A weaker run produces a weaker attestation.
10. **Toolchain identity.** First-party Omnia policy names PATH binaries (`rustc`, `cargo`, `rustfmt`, `clippy-driver`). Preflight captures version + executable digest into the bound context and the D8 cache key. Mid-attempt drift is `verification-tool-missing`. The published policy document does not pin a rustc semver.
11. **Fetch vs check.** Preflight may run a declared `fetch` (Omnia: `cargo fetch` to the granted registry). Profile runs are `--offline` (or equivalent). `CARGO_HOME` / target-dir under the lineage cache are class-1 mutable tool state. The content-addressed crate store, when used, is class-2 and verify-on-read.
12. **Mechanical-repair channel.** Phase A offers suggestions only from `fmt` (rustfmt whole-file replacements). Clippy `--fix` is not a Phase A channel.
13. **`source: tool` finding sibling.** Adapter-authored findings with `source: tool` fail as `target-phase-finding-source-tool` (finding granularity). Report-level `target-phase-source-tool` still rejects an adapter-returned `report` that claims `tool`. Engine-assembled reports from resolved host records may be `tool` or `hybrid`.

## Open questions

None. The decisions above are enough to implement Phase A without a richer surface. If a step hits a contradiction with RFC-97, stop and record it here before continuing.

## Coverage

RFC-97 implementation-requirement bullets (Phase A only) and acceptance criteria map to steps as follows.

| Requirement / criterion | Steps |
| --- | --- |
| Target metadata required profiles; `verify-result`; `emery:verification` import; hard cut; `emery-floor` | 3, 4, 5, 16, 17 |
| `project::seam::Verifier`; attempt-local attestations; mock scripted verifier | 2, 6, 7 |
| Capability crate on the `wasi-exec` shape; no exec-shaped WIT | 3 |
| Digest parity through the same snapshot kernel / blobstore convention | 11 |
| Genuine `emery:exec-mode` chmod on materialization | 11 |
| First-party Omnia policy registry, parsers, sandbox floor in the deployment-provider layer | 9, 10 |
| Preflight before the first slice build model call; closed D3 discriminants | 1, 8, 10 |
| Host-attested reports; oracle / execution assurance; assembly; reject adapter `source: tool` | 2, 7, 9 |
| Protected inputs / oracles on the admission-covered Node; capture-time ownership | 13 |
| Capture logical candidate before every `verify`; fresh RFC-87 workspace; lineage cache; cold confirmation | 6, 12 |
| D7 mechanical repair; persist `mechanical-repairs/<NNNN>.yaml`; journal the event | 14, 15 |
| Persist attestations under the attempt tree | 2, 7 |
| D9 per-profile events + `slice.build.mechanical-repair-completed` | 15 |
| AC1 host-backed Omnia; non-declaring targets unchanged | 7, 16, 17 |
| AC2 fail-closed D3; preflight before model spend | 8, 10, 18 |
| AC3 canonical normalization | 9, 18 |
| AC4 comparison predicates; RFC-90 budgets unchanged | 9, 14 |
| AC5 assurance classes; cold confirmation | 12, 13, 18 |
| AC6 mechanical repair keep/discard | 14, 18 |
| AC7 cache isolation | 12, 18 |
| AC8 D9 `slice-attempt` telemetry | 15, 18 |
| AC9 native + Wasm integration suites | 18 |
| AC10 protected-path validation and capture-time failure | 13, 18 |
| AC11 lineage-cache disposal | 12, 18 |
| AC12 `wasm-omnia-r9k` (operator-invoked) | 20 |

---

## Step 1 — Closed vocabulary and D3 discriminants

**Repository:** emery

**Goal.** Land the closed Phase A names and fail-closed error codes with no orchestration wiring.

**Work.**

- Add `project::verification` (or `project::seam::verification`) with:
  - `ProfileName` — closed enum `fmt | build | clippy | test | doc | vet | deny | ci`
  - `VerificationContextKind` — closed enum; Phase A accepts only `slice-attempt` (include the two domain variants in the enum so the wire field exists; reject them at bind)
  - `OracleAssurance` — `candidate | protected | mixed`
  - `ExecutionAssurance` — `model-assisted | host-attested | hybrid` (projected later; type only)
  - `SandboxFeature` — the D3 closed set
  - `ci_exclusive(profiles) -> bool` helper used by metadata resolution
- Route every D3 discriminant through `Error::Validation` (exit 2) or the existing generic path (exit 1) exactly as RFC-97 D3's table:

  | Discriminant | Exit |
  | --- | --- |
  | `verification-profile-unavailable` | 2 |
  | `verification-sandbox-denied` | 2 |
  | `verification-tool-missing` | 2 |
  | `verification-parser-missing` | 2 |
  | `verification-limit-exhausted` | 2 |
  | `verification-cancelled` | 1 |
  | `verification-platform-unsupported` | 2 |
  | `verification-attestation-mismatch` | 2 |
  | `verification-attestation-duplicate` | 2 |
  | `verification-attestation-persist-failed` | 1 |
  | `verification-profiles-incoherent` | 2 |

- Add `target-phase-finding-source-tool` as a `target-phase-*` sibling (finding granularity). Do not change `gate.rs` behaviour yet.
- Integration tests in `crates/project/tests/` for parse/display of the closed enums and for `ci` exclusivity (`fmt+ci` is incoherent; `ci` alone is not).

**Done when.** The types compile, the discriminants are stable kebab strings, and no verify/build path calls them yet.

**Do not.** Touch WIT, the phase machine, adapters, or journal events.

---

## Step 2 — Profile report, handle, and attempt-local store

**Repository:** emery

**Goal.** Closed DTOs for the host record and a persist/load helper at the attempt path.

**Work.**

- Add the profile-report DTO (kebab-case YAML, `deny_unknown_fields`):

  - `profile`, `platform`
  - `context` (`kind: slice-attempt` plus change / slice / attempt identity)
  - `candidate` snapshot id
  - `policy-digest`, `report-digest`
  - `oracle-assurance` (persisted)
  - `protected-inputs` / `oracles` digests actually bound by the executed policy (empty when none)
  - `enforced-sandbox` (the features actually enforced)
  - `toolchain-identity`
  - normalized `findings` (RFC-90 finding shape)
  - optional `suggestion-group` (`edits: [{ path, preimage-digest, result-digest }]`)
  - comparison predicates as host-computable functions over two reports, not stored fields: `unchanged_failure_set(a, b)`, `regression(candidate, best)`
  - raw fallback: `{ digest, tail }` when structured parse is absent — do not store volatile durations / pids / temp roots in the normalized body

- Handle type: opaque newtype over the report-bytes digest.
- Persist/load at `build/attempts/<attempt>/attestations/<handle>`. Failure to persist maps to `verification-attestation-persist-failed`.
- Tests: round-trip YAML; digest-stable under field reorder; comparison predicates on fixture reports; persist path layout.

**Done when.** Reports can be written and read without a verifier. Execution assurance is **not** a field on the report.

**Do not.** Run tools, assemble phase reports, or emit journal events.

---

## Step 3 — `emery:verification` package and `wasi-verification` crate

**Repository:** emery

**Goal.** Own the host capability the same way `wasi-exec` owns `emery:exec-mode`. Process execution stays native; no `wasi:exec`.

**Work.**

- New crate `crates/wasi-verification` (`emery-wasi-verification`, lib name `omnia_wasi_verification` so `omnia::runtime!` can derive `WasiVerification`).
- WIT at `crates/wasi-verification/wit/verification.wit`:

  ```wit
  package emery:verification@0.1.0;

  interface types { /* error variant mirroring D3 kebab codes + invalid-request / io / internal */ }

  interface verifier {
    run-profile: async func(profile: string, candidate: string) -> result<string, error>;
    resolve: func(handle: string) -> result<string, error>; // profile-report YAML bytes
  }

  interface control {
    bind: func(candidate: string, context: string) -> result<_, error>;
    preflight: func(context: string) -> result<_, error>;
  }

  world imports {
    import verifier;
    import control;
  }
  ```

  `candidate` is the opaque workspace id. `context` is engine-owned JSON/YAML of the pre-bound record (target, platform set, verification context, policy digest, protected-input handles). Adapters never see `control`.
- Vendor copies: `crates/guest/wit/deps/verification/` (symlink or copy, match `exec-mode` / `vcs`).
- Import `verifier` + `control` on `crates/guest/wit/engine.wit` `world workflow`.
- Host trampoline on the `wasi-exec` shape (`WasiVerification` + `VerificationDefault`). `run-profile` / `resolve` / `bind` / `preflight` return `verification-profile-unavailable` until later steps fill them in.
- Link `WasiVerification: VerificationDefault` in `src/main.rs` `omnia::runtime!` `hosts`.
- Workspace `Cargo.toml` already members `crates/*`. Add the native-only dependency on the root package beside `emery-wasi-exec`.
- Update `crates/guest/wit/deps/adapter/README.md` only if this step forces a wording clash; the adapter-world import lands in step 4.

**Done when.** The shipped binary links the new host. Engine guest WIT compiles. No adapter world change yet. No real tool execution.

**Do not.** Change `target.verify`. Do not add an exec-shaped WIT.

---

## Step 4 — Target-world hard cut (types only)

**Repository:** emery

**Goal.** Change the target contract to `verify-result` and required-profile metadata. Every in-tree implementor still returns `report(phase-report)`. Behaviour of the phase machine is unchanged.

**Work.**

- In `wit/emery.wit` (and the guest vendored `crates/guest/wit/deps/adapter/emery.wit`):
  - add `profile-set` + `required-profiles` on `adapter-metadata`
  - add `verification-answer` and `verify-result`
  - change `verify` to `result<verify-result, error>`
  - `world target-adapter` (and `world adapter`) import `emery:verification/verifier@0.1.0` — **not** `control`
- Adapter SDK: `TargetMetadata.required_profiles`, `VerifyResult { Report(PhaseReport) | Attested { attestations, findings } }`, `Target::verify` returns `VerifyResult`. `target!` export macro maps the new WIT. Add `adapter::verification::run_profile` / `resolve` wrappers over the import (they may still fail closed).
- Engine `project::seam::Target::verify` returns `VerifyResult`. Guest provider, native provider, mock `ops`, transport test doubles, `examples/wasm` target fixture: wrap today's `PhaseReport` as `VerifyResult::Report`.
- `Machine::verify_phase` unwraps `Report` and keeps today's path. `Attested` is a typed seam error for this step (`invalid-request` / `verification-attestation-mismatch` is fine — no target declares profiles yet).
- Existing `target-phase-source-tool` gate stays: adapter-returned `report` still cannot claim `tool`.
- Bump in-tree adapter `emery-floor` comments only if a crate in *this* repo declares one (mock / examples). First-party adapters bump in steps 16–17.

**Done when.** `cargo make test` subset for `adapter`, `project`, `slice`, `change`, `native`, `mock`, `guest` (host compile) is green. No host-attested Omnia path yet.

**Do not.** Assemble attestations. Do not change Omnia/contracts/vectis in `emery-adapters` (that is steps 16–17). Do not lift the report-level `tool` gate.

---

## Step 5 — Engine compile-through after the cut

**Repository:** emery

**Goal.** Finish any call sites step 4 missed so the workspace is clean before the verifier seam lands.

**Work.**

- Grep `fn verify` / `PhaseReport` verify returns across `crates/`, `examples/`, `docs/` comments that claim the old signature.
- Fix remaining test helpers (`crates/native/tests/support`, `crates/change/tests/build_phases`, `crates/adapter/tests/operations`).
- Keep mock verify on the `report` path (existing markers: `VERIFY_BLOCKED_MARKER`, etc.).

**Done when.** A focused `cargo nextest run -p emery-adapter -p emery-project -p emery-slice -p emery-change -p emery-native -p emery-mock` passes.

**Do not.** Add `Verifier` yet.

---

## Step 6 — `project::seam::Verifier` and the scripted mock

**Repository:** emery

**Goal.** The engine-facing capability beside `Workspaces`, with a scripted double for native tests.

**Work.**

- `project::seam::Verifier`:
  - `bind(candidate, context)`
  - `preflight(context)`
  - `run_profile(profile, candidate) -> Handle` (native tests; wasm adapters use the WIT import)
  - `resolve(handle) -> ProfileReport`
- Native provider: in-process implementation that delegates to a backend trait. Default backend still returns `verification-profile-unavailable` except when the scripted mock is installed.
- Guest provider: WIT `control` + `verifier` imports.
- `crates/mock`: scripted verifier keyed by profile name → fixture report or D3 discriminant. Same typed outcomes as the real host. Session helper to install it on the native provider.
- Tests in `crates/mock/tests/` or `crates/native/tests/`: bind → run → persist → resolve; mismatch / duplicate / missing handle fail closed.

**Done when.** Native tests can exercise attestation resolution without spawning Cargo. The phase machine still does not call `Verifier` on the happy path.

**Do not.** Implement sandbox or Omnia policy.

---

## Step 7 — Engine attested path and phase-report assembly

**Repository:** emery

**Goal.** Dual-mode `verify`: `report` stays byte-unchanged for non-declaring targets; a declaring target must return `attested`; the engine resolves handles and assembles one RFC-90 phase report.

**Work.**

- Immediately before every `verify` dispatch, capture the logical candidate (code snapshot + staged artifacts) and `Workspaces::prepare` a **fresh** RFC-87 workspace. Lend that workspace to `verify`. Bind `Verifier` against its id. Discard the verification workspace after the phase (the logical candidate is the snapshot, not the workspace id). Continuation continues to bind to the logical candidate / attempt continuation, not the verification workspace id.
- After `verify` returns:
  - `Report` — existing `gate::accept` path. If the target's metadata `required-profiles` is non-empty, fail (declaring target may not fall back).
  - `Attested` — resolve every handle via `Verifier::resolve`; check exact profile, platform, context, candidate, policy coverage; reject duplicates and extras; reject adapter findings with `source: tool` as `target-phase-finding-source-tool`.
  - Project findings in required-profile order, then one RFC-90 D2 canonicalize over the union.
  - Report-level `phase-source`: `tool` when only host records contribute; `hybrid` when deterministic in-component findings also contribute. Lift `target-phase-source-tool` **only** for this engine-assembled report.
  - Project execution assurance (`host-attested` / `hybrid`) for later telemetry; do not store it on the profile report.
- Persist resolved reports under the attempt `attestations/` tree if the host has not already (host persist-before-return is the authority; engine must not mint reports).
- Tests with the scripted verifier: complete set → `phase-source: tool`; missing profile; duplicate handle; forged/mismatched candidate; non-declaring mock still uses `report`; declaring mock cannot return `report`.

**Done when.** The serial RFC-90 loop can complete a host-attested verify against the mock without a model call on that phase. Repair budgets are unchanged.

**Do not.** Preflight real tools. Do not implement D7 or D8.

---

## Step 8 — Slice preflight before the first build model call

**Repository:** emery

**Goal.** Fail closed before build model spend when a required profile cannot run.

**Work.**

- At the start of a slice build attempt — after target resolve, **before** `Machine::build_phase` (the first model call) — run `Verifier::preflight` for every required `(platform, profile)` of the bound target.
- Preflight checks, as the backend grows: policy present, `ci` exclusivity, parser registered, sandbox features enforceable, toolchain identity, admission-covered protected inputs the policy will mount, unsupported tuples.
- This step wires the call and the incoherent / unavailable / unsupported discriminants against the scripted backend (script a missing policy, an incoherent `ci` set, an unsupported platform).
- Domain preflight is Phase B — do not add it.

**Done when.** A scripted preflight failure aborts the attempt with a D3 discriminant and no `target.build` model dispatch. Existing non-declaring targets skip preflight.

**Do not.** Implement the real Omnia registry (step 9) or process spawn (step 10).

---

## Step 9 — Omnia policy registry, parsers, and comparison predicates

**Repository:** emery

**Goal.** Deployment-owned policy data and deterministic normalization. Engine crates gain no `if target == "omnia"` branch.

**Work.**

- Place the first-party registry in the deployment-provider layer (`crates/wasi-verification` and/or the root `launcher` module, `src/launcher.rs`), keyed by `(target name, platform, profile)`.
- First-cut Omnia/`core` commands (daemonless, check-only — no source mutation):

  | Profile | Command sketch |
  | --- | --- |
  | `fmt` | `cargo fmt --check --all` |
  | `build` | `cargo check --locked --workspace --all-targets --all-features` |
  | `clippy` | `cargo clippy --locked --workspace --all-targets --all-features --message-format=json -- -D warnings` |
  | `test` | `cargo nextest run --locked --workspace --all-features --no-tests=pass` (raw-fallback if nextest is absent and policy requires it → `verification-tool-missing`) |
  | `doc` / `vet` / `deny` / `ci` | Registered or explicitly absent; absent → `verification-profile-unavailable` / `verification-parser-missing` |

- Each policy names: argv, env allowlist, ephemeral write roots (`CARGO_HOME`, `CARGO_TARGET_DIR` under the lineage cache), granted `fetch` destinations, parser id, optional `fmt` suggestion channel (`rustfmt` emit or equivalent whole-file replacements), resource limits.
- Policy digest is over the canonical policy document.
- Parsers: structured tool JSON preferred; raw fallback is digest + secret-filtered tail. Strip durations, pids, tids, temp roots; paths become candidate-relative `/`; cascade suppression is profile-defined and must not drop an independent blocking root; fingerprints and RFC-90 sort key.
- Tests: equivalent structured output with different durations / pids / order → byte-identical normalized reports and fingerprints (AC3). Comparison predicates stable (AC4). Command-injection: policy argv is fixed; adapter-supplied argv cannot exist on this API — assert `run-profile` accepts only a profile name.

**Done when.** A fixture clippy/rustc JSON blob normalizes identically twice. No process spawn required (feed parsers from files).

**Do not.** Spawn tools yet. Do not put the registry in `crates/slice` or `crates/change`.

---

## Step 10 — Native execution and portable sandbox

**Repository:** emery

**Goal.** The host runs the pre-bound policy in the candidate workspace under D3's denied-by-default rules.

**Work.**

- `VerificationDefault::run_profile`: look up bind context by workspace id; select policy; re-verify toolchain identity; spawn in the candidate workdir; apply env allowlist (no inherited credentials); bound wall time / stdio / process count; reap the process tree on timeout or cancel; writes only to declared ephemeral roots; mount protected inputs read-only when the policy binds them.
- Network during the check is denied when enforceable; otherwise do not grant `HTTP_PROXY` / `ALL_PROXY` / cargo net config, and pass `--offline` for Cargo checks. `fetch` is a separate preflight step, not a profile.
- Persist the normalized report **before** returning the handle. Persist failure is `verification-attestation-persist-failed`.
- Map spawn outcomes onto D3 discriminants (`verification-limit-exhausted`, `verification-cancelled`, `verification-tool-missing`, `verification-sandbox-denied`).
- Tests (native, not Wasm): environment and egress denial (no leaked `CARGO_REGISTRIES_*` / tokens in the child env); resource-limit timeout; process-tree reap; protected-input write denial; command-injection refusal (profile name `"fmt; touch pwned"` is an unknown profile, not a shell).

**Done when.** A tiny fixture crate in a prepared workspace can run `fmt` or `build` through the host and produce a persisted attestation. Engine crates still have no Omnia branch.

**Do not.** Implement warm cache or mechanical repair.

---

## Step 11 — Digest parity and executable bits

**Repository:** emery

**Goal.** Attestations bind candidate snapshot ids that the engine guest and the native verifier compute the same way. A guest-marked executable is executable when the verifier runs it.

**Work.**

- Native verifier reaches objects through the same workspace kernel (`project::workspace::Store`) and the same `<2 hex>/<62 hex>` layout the launcher blobstore uses (`$EMERY_HOME/snapshots/`). Do not invent a second hasher.
- Test: engine-side `snapshot` / `prepare` of a fixture tree; host-side recompute of the id; they match.
- Confirm `emery:exec-mode` / `FsExecMode::apply` still performs genuine `chmod` on Unix. Add an integration test: snapshot a `100755` script, materialize, native verifier (or a direct `std::fs` exec check) can execute it. No-op on non-Unix stays as today.

**Done when.** A mismatched-id test would fail; the chmod test fails if materialization drops `+x`.

**Do not.** Change the snapshot manifest format.

---

## Step 12 — Lineage cache and cold confirmation

**Repository:** emery

**Goal.** D8: warm incremental tool state inside one slice-attempt lineage; cold report is authority.

**Work.**

- Cache key = context kind+id, target name+version, platform, policy digest, toolchain identity, sandbox/env-policy digest.
- Mount or copy the private cache into the verification workspace as ephemeral write roots. Never a shared writable product tree. Never a cached verdict.
- Every warm **passing** required profile reruns once with an empty cache before it may contribute to a successful verify phase. The cold report is authority.
- When a warm required profile fails **and** the attempt would terminate (verification-repair budget exhausted), rerun that profile cold once before minting the terminal report. Warm failures may still iterate.
- Dispose the lineage cache on attempt completion, abandonment, policy-digest change, and a simple age/GC sweep (attempt-scoped is enough; do not build a general GC product).
- Tests: successive repair candidates in one lineage see a warm cache; a second context / target / platform / toolchain / policy does not; cache files are absent from `capture`; a warm pass that fails cold does not gate; a warm-only failure is not terminal without the cold rerun (AC5, AC7, AC11).

**Done when.** Scripted or fixture-tool tests prove isolation and cold confirmation. Domain cache reuse is not implemented.

**Do not.** Add D7 yet.

---

## Step 13 — Protected inputs and capture-time ownership

**Repository:** emery

**Goal.** D4 / AC10 on the admission-covered Node. A declaration alone never upgrades oracle assurance.

**Work.**

- Metadata optional `default-covered` / `default-oracles` (same shapes as `Covered` / `Oracle`).
- `plan author` copies defaults onto leaves that have empty `covered` / `oracles`.
- `plan amend` flags to set or clear those fields on a named entry; reproject through `decomposition.yaml` when it exists (`plan-mutation-ambiguous` otherwise, same as today's topology amend).
- `emery slice validate` / `plan validate`: orphan protected paths (outside the leaf ownership envelope); grant/protection overlap (`writable-artifacts[]` vs `covered`).
- Capture-time: if captured touched paths intersect the admission-covered protected set, fail the attempt with a typed ownership finding (new `target-phase-*` / `verification-*` finding code — pick one family and use it consistently; prefer a verification ownership finding, not an RFC-87 mount-exclusion).
- Host derives `oracle-assurance` from the **executed** policy's input contract and the digest-matched mounts. No bind → `candidate`. A test policy that mounts an admission-covered in-tree file → `protected` or `mixed`.
- Tests: validate orphans and overlap; capture-time intersection fails; declaration without a binding policy stays `candidate`; writers other than author/amend cannot introduce the fields (grep other plan handlers).

**Done when.** AC10 holds on native tests. RFC-98 corpus oracles are not produced.

**Do not.** Implement RFC-96 domain closure.

---

## Step 14 — Host mechanical repair (D7)

**Repository:** emery

**Goal.** One explicit slice-only phase between a failed verify and RFC-90 model repair. Not a target operation; no phase ordinal.

**Work.**

- After a failed slice verify, if exactly one eligible suggestion group exists (required-profile order, then group digest): every edit from one attested report, exact candidate preimage, inside the slice write grant, no protected path, ≤ 16 edits, each file ≤ 1 MiB, group ≤ 4 MiB.
- Apply the group in a **fresh** workspace; capture a tentative snapshot; rerun every required profile (warm allowed; keep/discard uses the warm rerun).
- Keep only when the originating profile's blocking fingerprint set strictly shrinks **and** `regression(candidate, best)` is false for every required profile. Otherwise discard and route the **original** findings to model repair. The fix consumes no model-repair dispatch and cannot trigger another mechanical repair before the next model repair.
- On keep: the tentative snapshot becomes the logical candidate through the RFC-87 capture/materialize boundary. A clean report advances to review; remaining blocking findings go to model repair.
- Persist `build/attempts/<attempt>/mechanical-repairs/<NNNN>.yaml` (zero-padded ordinal) with grant, patch, source/tentative snapshot ids, before/after report digests, `rejected | accepted`.
- Ineligible / stale / partial / over-bound groups leave the source candidate unchanged and do not consume a model repair.
- Domain verification never offers this phase (no code path in Phase A anyway).
- Tests: keep, discard-on-regression, stale preimage, out-of-grant, protected path, over-bound, unchanged set (AC6). RFC-90 verification-repair count is unchanged when the group is rejected.

**Done when.** The machine inserts at most one D7 attempt per failed verify, then model repair as today.

**Do not.** Add a fix queue or a second mechanical pass.

---

## Step 15 — D9 telemetry

**Repository:** emery

**Goal.** Raw evidence events. Nothing here changes lifecycle or report success.

**Work.**

- Add `EventKind` variants:
  - `target.verify.profile-completed` with the RFC-97 D9 field list (`slice-attempt` only; include `kind` so Phase B can open later). `run-kind: primary | cold-confirmation`. `cache-disposition: disabled | cold | warm`. `mechanical-repair-disposition` on the profile event is observational (`not-offered | rejected | accepted`) and does not imply write authority.
  - `slice.build.mechanical-repair-completed` with source/tentative snapshot ids and `rejected | accepted`.
- Emit one profile event per profile execution (including cold confirmations). Emit the mechanical-repair event from the D7 phase.
- Update `docs/standards/cli-contract.md` event table and `docs/standards/workflow.md` if it lists the closed taxonomy. Keep `slice.build.phase-completed` as the RFC-90 envelope.
- Tests: journal fixtures for a host-attested verify emit the expected profile events; fingerprints / lifecycle projection ignore timing and cache fields.

**Done when.** AC8 holds for Phase A slice events. No metric drives a gate.

**Do not.** Add domain-context events.

---

## Step 16 — Omnia adapter: declare and request profiles

**Repository:** emery-adapters (path-patch to the emery branch)

**Goal.** Omnia `verify` is deterministic host-profile requests. No model, no argv.

**Work.**

- `TargetMetadata.required_profiles` = one `core` set `[fmt, build, clippy, test]`.
- Bump `emery-floor` to the engine workspace version that contains the cut (today's in-tree floor is `0.38.0`; use the version on the emery branch).
- `verify`: for each required profile name (and platform slot), call `adapter::verification::run_profile`; return `VerifyResult::Attested` with the handles and any deterministic in-component findings (`source` not `tool`).
- Delete or retire the model-assisted verify prompt path for Omnia (the prompt may remain on disk unused; do not leave a silent fallback).
- Native adapter tests: metadata shape; verify returns `attested` against a scripted verifier; no model call.

**Done when.** Omnia cannot return `report` while it declares profiles. AC1's adapter half is done; live `wasm-omnia-r9k` is step 20.

**Do not.** Change contracts or vectis in this step.

---

## Step 17 — Contracts and Vectis type-wrapper

**Repository:** emery-adapters

**Goal.** Non-declaring targets keep today's `phase-report` payload byte-unchanged.

**Work.**

- Return `VerifyResult::Report(existing_phase_report)`.
- `required_profiles` empty.
- Bump `emery-floor` to the same engine version as step 16.
- Update verify tests to the new return type. No prompt or command changes.

**Done when.** Contracts and vectis still exercise the model-assisted / deterministic verify they have today, wrapped as `report`.

**Do not.** Declare host profiles for these targets.

---

## Step 18 — Phase A integration suite

**Repository:** emery (and adapters tests already added)

**Goal.** AC9 Phase A coverage on native and Wasm fixtures, without a live model.

**Work.**

- Native (mock catalog + scripted verifier + fixture tools where needed):
  - direct attestation resolution
  - profile-set completeness
  - command-injection refusal
  - environment and egress denial
  - resource limits
  - process-tree cancellation
  - protected-input write denial
  - canonical parsing and raw-fallback bounds
  - slice mechanical rollback
  - cold confirmation for `candidate` and `protected` / `mixed` (script a binding policy)
  - cache isolation
  - D9 telemetry
  - preflight before build model spend
- Wasm: update `examples/wasm` target fixture for `verify-result`. Add a fixture that declares a profile and relays a scripted host handle if the wasm example host can install `WasiVerification`. If the wasm example cannot host a real verifier in this cut, cover the WIT signature and `report` path there, and keep host-attested coverage on native — do not invent a second Omnia host.
- Run the named nextest packages. Do not run `cargo make eval` or `wasm-omnia-r9k` here.

**Done when.** AC9's Phase A list is asserted. Phase B domain no-repair / closure tests are not written.

---

## Step 19 — Docs and crate-graph prose

**Repository:** emery

**Goal.** The spine matches the code. A contributor can reach the new seam in three hops.

**Work.** Update in the same change (grep the old verify signature and `target-phase-source-tool` “until RFC-97” wording):

- `AGENTS.md` — crate graph (`wasi-verification`), `Verifier` beside `Workspaces`, dual-mode verify, D3 discriminants, D7/D9 event names, Omnia host verification
- `docs/standards/workflow.md` — verify dual-mode; host profiles
- `docs/standards/cli-contract.md` — new events
- `docs/reference/adapter-contract.md` — `required-profiles`, `verify-result`, `emery:verification` import
- `docs/reference/targets/omnia.md` — host-attested verify
- `docs/reference/diagnostics.md` — D3 codes and ownership finding
- `docs/reference/targets/index.md` if it still says verify is always model-assisted
- `crates/guest/wit/deps/adapter/README.md` — target world now imports `emery:verification`; the package is no longer import-free
- `docs/standards/architecture.md` if it lists host capability crates

Run `cargo make links` if any Developer Guide path changed.

**Done when.** Grep for the old `result<phase-report, error>` verify story in `AGENTS.md` and `docs/` is clean (RFCs may keep historical wording).

**Do not.** Rewrite RFC-97.

---

## Step 20 — Operator-invoked `wasm-omnia-r9k` (checklist)

**Repository:** both (no code unless the run exposes a Phase A bug)

**Goal.** AC12. This step is operator-run, not CI.

**Work.**

- Path-patched adapters + the emery branch binary.
- `cargo make wasm-omnia-r9k` (or its successor) in `emery-adapters`.
- Confirm no final-grade regression against the last model-assisted baseline.
- From D9 events, record verification wall-clock and cold-confirmation overhead versus that baseline (a short note in the PR body is enough; do not add a metrics product).

**Done when.** The operator has a pass/fail and the two numbers. If it fails, fix on the same branches (do not open a second plan).

---

## Suggested session order

```text
emery:        1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 → 12 → 13 → 14 → 15
adapters:                          (path patch after 4)     16 → 17
emery:        18 → 19
operator:     20, then one PR on emery and one PR on emery-adapters
```

Steps 16–17 may start after step 4 (type-wrapper) but Omnia must not declare profiles until steps 9–10 exist, or preflight will fail closed. Prefer starting 16 after 15.

## Out of scope (do not sneak in)

- Phase B domain contexts, domain repair, domain cache, RFC-96 D8 closure derivation
- RFC-98 `conserve` / corpus oracles
- RFC-92 route-escalation consumption of `unchanged-failure-set` (the predicates exist; the route policy does not change)
- Clippy `--fix` as a mechanical-repair channel
- A host-global attestation store
- `wasi:exec` or any exec-shaped WIT
- Publishing wasm-pkg packages or GHCR adapter tags
- Git commits and pull requests
