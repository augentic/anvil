# Portable storage and the human seam

Status: **Draft** — working design, executed as the discrete steps in [§7](#7-execution-steps). Each step lands independently with the journey test green.

## 1. Motivation

The engine's persistence today is the host filesystem, wired through a static, CWD-rooted deployment policy in `src/main.rs`: the invocation directory mounts writable as `.`, and a CWD-relative `.emery-cache` backs the cache preopen. That policy hard-codes three constraints we want to remove:

- **One project per deployment.** `crates/engine/src/handler/locations.rs` notes explicitly that no project-id keying is needed *because* the cache is CWD-relative. Multi-project, multi-tenant, and serverless deployments are impossible without re-keying storage.
- **A writable `.` mount.** The guest holds write authority over the operator's entire project tree in order to maintain one subtree (`.emery/`). Least-authority (the C3 posture) wants the guest holding exactly the capabilities it uses.
- **Hand-rolled durability.** `crates/artifacts/src/atomic.rs` (temp file, `sync_all`, atomic rename) and the crash-litter pruning in `Home::prune` exist only because the filesystem offers no better primitive.

The move: engine state goes behind `wasi:keyvalue` and `wasi:blobstore` capability imports, with the host binding deciding what backs them (local directory, sqlite, object store, …). Deployment becomes host policy, not engine code.

This is tractable because the output home is already store-shaped: `crates/engine/src/home.rs` writes immutable, digest-named generation sets, swaps one small `current` pointer, and prunes what the pointer no longer names. That is exactly the idiom a keyvalue-pointer-over-immutable-blobs store requires — the port changes the backing, not the semantics.

## 2. Goals and non-goals

Goals:

1. Engine state (generation store, pointer, component cache, global store, locks) reachable only through storage capabilities; no `std::fs` against engine-owned state in guest code.
2. The host binding chooses the backing. The shipped local binary defaults to a durable filesystem store under the invocation directory (generations survive restart the way `.emery/spec/` does today). That is store durability, not an unchanged working tree — `specify` no longer writing `spec.md` into the tree ([step 6](#7-execution-steps)) is an intentional operator-visible change.
3. The human seam — review of `spec.md` / `design.md` — survives as a **verifiable, non-authoritative projection** of the store, via the `specify` envelope and `emery show` ([§4](#4-the-human-seam)). Generation is not repo-anchored (§4 standing fact 3); git enters at delivery, not here.
4. Net deletion where the backing permits it: `atomic.rs`, the litter half of `prune`, the `cache_dir()` mkdir in `main.rs`, the writable `.` mount, `.emery/project.yaml` ([§5](#5-deleting-projectyaml)), and `bindings.yaml` / `receipts.yaml` from the generation set all go.

Non-goals:

- Moving the **workspace lend**. The WIT `workspace` record (`wit/emery.wit`) lends the operator's live source tree read-only to the model; it is inherently a filesystem view and stays one. (A content-addressed workspace snapshot for reproducible extraction is a separate, larger design.)
- Dynamic adapter resolution or any download path.
- A migration framework. Pre-1.0, crossing this boundary is a re-init.
- A human-edited replacement config discovered implicitly (`emery.yaml` at the repo root, etc.). That reverses the rule that the CLI is the only mutation path and recreates the same noun. Distinct from the explicit `--sources sources.toml` argv carrier ([§5.2](#52-replacement-authorities)), which the engine never writes or discovers.
- `emery diff`, `emery materialize`, and exposing extract receipts or bindings on `show`. The re-mine diff stays on the `specify` envelope (exists). Review is `show spec|design` to stdout. A working-tree copy of the spec is a delivery-loop concern ([layer 3](#layer-3--git-projection)), not a generate-time verb. `bindings.yaml` and `receipts.yaml` are deleted: both were generation snapshots nothing reads back. The adapter list is argv / `sources.toml` (option 1); requirement-level provenance stays `Sources:` in `spec.md`. The generation is the two reviewable documents.

## 3. Target architecture

### 3.1 Storage inventory and destination

| Surface | Today | Destination |
| --- | --- | --- |
| Generation documents (`spec.md`, `design.md`) | `.emery/spec/generations/<digest>/` | blobstore container, objects keyed by generation digest. `bindings.yaml` and `receipts.yaml` **deleted** — snapshots nothing reads back |
| `current` pointer | `.emery/spec/current` file | keyvalue entry; swap is one atomic `set` |
| Component cache | `.emery-cache/components/<name>.wasm` preopen | blobstore container keyed by adapter name |
| Global adapter store + `.meta` sidecars | host-side `<store>/<name>@<version>.wasm` + sidecar | blobstore (immutable entries) + keyvalue (provenance sidecars) |
| Locks / PID stamps | file stamps via `bytes_write` | keyvalue atomics (CAS / increment) |
| `project.yaml` | `.emery/project.yaml` | **deleted** — the file is a sticky copy of `init`'s argv, not engine state ([§5](#5-deleting-projectyaml)) |
| Workspace lend | read-only view of the source tree | **stays on disk** (non-goal) |

### 3.2 Authority model

- The **keyvalue pointer is the single authority** for "what is the current generation". A read failure on the pointer is an error, never an empty result — exactly the posture `Home::current` already takes with `spec-home-corrupt`.
- Generation blobs are **immutable and self-verifying**: `SpecSet::id()` is the digest of the documents' bytes, so any copy anywhere can be checked against the pointer. Projections ([§4](#4-the-human-seam)) are therefore never trusted, only verified.
- Multi-document commit needs no transaction: write the immutable blobs first, then swap the one pointer key — the ordering `Home::commit` already implements over the filesystem.

### 3.3 The capability seam

Follow the existing provider pattern (`omnia_guest::Model`, `emery_adapter::Source`; see the bare `Provider` in `src/lib.rs`): one engine-side storage capability trait pair, with wasm32 defaults over the `wasi:keyvalue` / `wasi:blobstore` imports and bare native impls so tests script storage in memory exactly as they script the model and the source seam today.

omnia-guest already ships this exact shape for storage: `omnia_guest::StateStore` and `omnia_guest::BlobStore` are pre-existing capability traits whose wasm32 defaults delegate to the `omnia-wasi-keyvalue` / `omnia-wasi-blobstore` imports. The engine seam consumes or mirrors those traits rather than inventing a new pair.

`emery_engine::home` remains the one module owning spec-set reads/writes; `Locations` stops being path math and becomes key/container-name math. Kernels keep consuming values and never touch the environment.

### 3.4 Host bindings

The `omnia::runtime!` invocation in `src/main.rs` grows `WasiKeyvalue` / `WasiBlobstore` host entries beside `WasiHttp` / `WasiModel`, and the `mounts:` table shrinks (cache mount deleted; `.` drops to read-only in step 6, since `specify` stops writing the working tree). The shipped binary's default binding is a durable filesystem store under the invocation directory, so a local restart still has `current` — the durability of today's `.emery/spec/`, not a promise that `specify` still drops `spec.md` in the working tree (step 6). Alternative bindings (project-id-keyed, remote) are deployment profiles, not engine changes.

> [!NOTE]
> `wasi-keyvalue` and `wasi-blobstore` are early-phase WASI proposals — WITs are unstable and stock wasmtime does not ship host implementations — but omnia already mitigates both halves. It vendors its own fork of each WIT (`omnia/crates/wasi-keyvalue/wit`, `omnia/crates/wasi-blobstore/wit`) and ships the host implementations (`WasiKeyValue` / `WasiBlobstore`, in-memory defaults) plus the guest capabilities (§3.3). The `omnia-backends` repo carries a pre-existing filesystem backend for wasi-blobstore (`omnia-filesystem`; durable, network-free) as a one-line host swap. Upstream churn lands against the omnia fork as a versioned seam change, like the adapter WIT.

## 4. The human seam

The spec is a derived document. The loop this work preserves:

1. **Mine.** `emery specify <adapter>...` or `emery specify --sources sources.toml`. Extract, reconcile, synthesise, commit a generation (immutable blobs, then the `current` pointer).
2. **Review.** The success envelope (generation id, re-mine diff); then `emery show spec|design`. Later the MCP resource and a delivery PR. Pipe `show` to a pager if you want an editor.
3. **Change.** Edit a *source* — intent `--value` or the `value` field in `sources.toml`, the workspace those adapters extract, or the adapter list — and re-run `specify`. There is no load/patch/save of `spec.md`. `show` is stdout; there is no working-tree copy to edit.

Durability is three scopes, not one:

- **Same deployment, next process.** The store. The local binary's filesystem binding keeps `current` across restart the way `.emery/spec/` does today. No file in the working tree does not mean ephemeral.
- **Fresh clone / CI.** Re-run `specify` from remembered sources (a committed `sources.toml` passed explicitly, or adapters in the Makefile/skill). The spec is regenerated, not checked out.
- **Spec as a git deliverable.** [Layer 3](#layer-3--git-projection) — the delivery loop copies the specs into a checkout it owns — not generation time.

Three standing facts shrink the rest:

1. **The filesystem is already read-only to humans.** The CLI contract forbids hand-editing anything under `.emery/`; every mutation routes through the CLI. The seam to preserve is *review* — read, never edit of the generated documents. Mutation is the loop above: sources in, `specify` again.
2. **Every generation is self-verifying** (§3.2), so we can hand out any number of non-authoritative views without forking authority.
3. **Generation is not repo-anchored.** An earlier draft treated *the spec is in the repo when you clone it* as a load-bearing product property. It is not: when code generation returns (the `v1`-tagged loop), delivery creates a temporary checkout, generates code, adds the specs, commits, and raises a PR. The spec reaches a repo at delivery time, in a checkout the pipeline owns — never at generation time in whatever directory `specify` happened to run. Between generation and delivery, the store is the spec's only home, and the store backing carries the durability weight a git-tracked copy used to.

The seam is delivered in layers over one authority. Layer 1 is part of this work's definition of done; layers 2–3 are follow-on deployment profiles. Deleting `init` ([§5](#5-deleting-projectyaml) option 1) is the named deletion for `show` (constitution invariant 2). Net live verbs: `specify` and `show`; `completions` stays auto-derived. That is a policy change in the PR that adds `show` (invariant 5).

### Layer 0 — the envelope (exists)

The `specify` envelope is already the reporting channel: the re-mine diff is emitted, never persisted. `show` follows that precedent — rendered from the store, emitted, never a second authority. Superseded generations stay pruned on pointer swap, as today; the envelope computes the diff in memory before prune. On-demand history is a layer-3 concern, not a verb.

### Layer 1 — `emery show`

The CLI grows one read verb over the store:

- `emery show spec|design` — render that document of the current generation to stdout, with the generation id in the envelope.

Those are the reviewable documents — and the whole generation. There is no `show bindings` or `show receipts`: `bindings.yaml` and `receipts.yaml` are deleted. The adapter list is this run's argv or `sources.toml`; requirement-level provenance is `Sources:` in `spec.md`.

`specify` stops writing `spec.md` / `design.md` into the working tree (step 6). Review after that is the envelope plus `show`. This supersedes the earlier stance that a default working-tree checkout preserved a load-bearing "spec is in the repo" property (standing fact 3 above): the future delivery loop re-homes specs into its own temporary checkout and PR, so nothing rides on the generation-time working tree holding a copy.

### Layer 2 — read-only MCP resource

The pre-bound listener already serves MCP reference shelves only, with the typed C3 refusal (`crates/transport/src/http.rs`) rejecting everything else. A read-only resource exposing the current generation and its id fits that posture — reads were never what C3 fences — and serves the growing consumer that is not a human at a shell: IDEs and agents (the `plugins/emery/` skill included).

### Layer 3 — git projection

This is the sketch of the actual delivery path when code generation returns: the loop creates a temporary checkout, generates code, copies the specs beside it, commits, and raises a PR — review becomes code review at delivery. The heavier variant — a blobstore host binding backed by a git object database — additionally yields generation history for free. Deferred; recorded here so the container/key naming in step 3 does not preclude it.

## 5. Deleting `project.yaml`

`.emery/project.yaml` is the spec generator's authored project record: identity, the CLI version pin, and the source bindings `specify` extracts from. Written only by `emery init`; loaded fail-closed by `specify` via `RequestContext`. Humans must not edit it.

It is not a twin of the generation store. The store is output (`spec.md`, `design.md`). `project.yaml` is a **sticky copy of `init`'s argv** — which adapters, under which key, workspace vs `--value`. That list is not re-homed into the generation: `bindings.yaml` is deleted with this work. Later runs take adapters from argv / `sources.toml` (option 1).

**Decision: the file is deleted**, not kept as a disk residue of this work (supersedes the earlier "stays on disk / travels with the repo" stance). This repo already gitignores `.emery/project.yaml`, so clone-and-specify from git alone is not current practice here; the "belongs in the repo" claim was design intent, not a load-bearing property.

### 5.1 Jobs to re-home or drop

| Outcome | Today | After deletion |
| --- | --- | --- |
| Which adapters to extract, and workspace vs `--value` | `sources:` | Re-homed — this is `specify`'s input. Three authorities below. |
| "Has init run?" | file exists → else `not-initialized` | Follows the chosen authority; dies if `specify` no longer needs a prior step |
| Project version floor | `emery:` pin → exit 3 | **Dropped.** Adapter `emery-floor` already refuses a too-old binary (`AdapterCliTooOld`). A second pin is how the file justified itself as a compatibility document. |
| `init --upgrade` re-ensures without rewriting bindings | reload file, bump pin | Follows wherever bindings live; dies with `init` under option 1 |
| `name` / `description` | written, never read by extract / reconcile / synthesise | **Dropped.** Unused. |

### 5.2 Replacement authorities

Three ways to keep the operator journey without the file. Option 3 keeps the *concept* and deletes only the YAML path. Option 1 removes the concept. Option 2 is rejected.

**Option 1 — collapse `init` into `specify` (preferred).** `emery specify <adapter>... [--value <adapter>=<text>] | --sources <path>`. Ensure, extract, commit in one verb.

- Persistence of the binding list goes away. Repeat the adapters every run (Makefile, skill, CI) — or point at an operator-owned `sources.toml` (below).
- `not-initialized` dies. There is no project record to be missing.
- `--upgrade` dies. Re-running `specify` re-ensures as a side effect of resolve, the way extract already re-resolves.
- Local `.wasm` mirroring moves to the first `specify` (today that is `ensure` inside `init`).
- Changing sources is a different argv. Intent text (`--value`) is passed every time unless the operator points at a file they own.
- Constitution impact: deleting `init` is the named deletion for `show`; the live-verb ledger is in §4. The `/emery:init` skill wraps `specify` or is deleted with `init`.

This is the deletion-aligned option: one generate verb, no project-config noun, no sticky file. Its stated cost — the adapter list is no longer remembered inside the tree — is removed by the file carrier below.

**Option 1's file carrier — `sources.toml` via `--sources <path>`.** The operator may write the binding list in a file they own and pass it explicitly: `emery specify --sources sources.toml`. The precedent is v1's hand-authored definition home (`scope.yaml` + `coverage.yaml`, no `system init`, archived at the `v1` tag): an operator-authored input file, loaded fail-closed with a schema version and typed errors, never engine-written. Rules:

- **Explicit `--sources <path>` only at this stage.** No implicit `./sources.toml` discovery — discovery is what turns a file into a config noun; the explicit flag keeps it an argv macro. Loosening to a default lookup would be a deliberate later decision.
- **The engine never writes it.** Mutation authority stays two-pole: engine state mutates only through the CLI; `sources.toml` mutates only in the operator's editor. The file is never read back between runs except when passed again.
- **Mixing refuses typed.** Positional adapters (or `--value`) together with `--sources` is `Error::Argument` (exit 2). Merge/override semantics can be added later if a need shows up.
- **TOML is deliberate.** Human-centric configuration files shift gradually to TOML — the idiomatic choice for Rust codebases — while engine-written artifacts stay YAML. Cost acknowledged: one new parse dependency beside `serde-saphyr`.

Draft schema, to be refined:

```toml
# sources.toml — operator-authored; the engine never writes this file.
# Passed explicitly: `emery specify --sources sources.toml`.
version = 1

# Package reference, extracting over the workspace lend (the default).
[[source]]
adapter = "emery:documentation@1.2.0"

# Bare name (first-party shorthand): newest published version.
[[source]]
adapter = "typescript"

# Inline value instead of the workspace — the file form of `--value`.
[[source]]
adapter = "intent@1.0.0"
value = "Ship a location-independent spec generator."

# Local component path, mirrored into the cache at ensure.
[[source]]
adapter = "./adapters/custom.wasm"
```

**Option 2 — rejected.** Reuse `current`'s binding list on a later `specify` with no adapters. That requires a stored argv snapshot in the generation (`bindings.yaml`) — the same sticky copy as `project.yaml`. This work deletes that file; option 2 is unavailable. Folding unused argv into the generation digest would also fork identity when `spec.md` did not change.

**Option 3 — bindings live in the store, not on disk.** `init` writes a keyvalue entry (selectors + values). `specify` reads it. The YAML file is gone; the data is not.

- Initialized = that key exists.
- `--upgrade` re-ensures from the key.
- Step 1 already wants this shape for engine state.
- Clone/CI still need a host binding that survives `git clone` (filesystem-backed store under the repo). Otherwise a fresh checkout cannot `specify` without re-passing adapters — which collapses to option 1.

This is a rename of `project.yaml` into the store. Worth it only if the store is coming anyway and a git-tracked file is refused, but the clone story still demands a host binding that survives checkout.

**Rejected:** a human-edited `emery.yaml` at the repo root, discovered implicitly as project config. Same noun, inverted mutation rule. `sources.toml` survives this rejection on both counts: it is an explicit argv input, not a discovered record, and the engine never writes it.

### 5.3 The gating question

After `git clone`, how does `specify` know the adapters?

- Remembered in argv / CI → option 1, and `init` can go. The argv may point at a committed `sources.toml` (§5.2), which is how a clone remembers the bindings without an engine-owned record.
- Remembered in host storage → option 3, and the local binary's filesystem binding has to look like today's `.emery/` to the operator.

Option 1 plus adapter-floor-only versioning is the one that does not leave a `project.yaml`-shaped residue for step 6 and `atomic.rs` to worry about; with the `sources.toml` carrier it also answers the clone question directly. Option 3 is recorded in case a requirement emerges that a clone must `specify` with no argv at all; it is not the default. Reusing the last generation's argv (former option 2) is rejected: it needs `bindings.yaml`.

## 6. Deletions

- `crates/artifacts/src/atomic.rs` — blobstore writes are complete-on-finalize; the keyvalue `set` is atomic. No working-tree write remains, so the crate goes outright. `project.yaml` is not a surviving call site.
- The crash-litter half of `Home::prune` — no temp files exist to leak. Superseded-generation prune on pointer swap stays (today's semantics).
- `cache_dir()` and its `create_dir_all` in `src/main.rs`; the cache mount and `GUEST_CACHE_MOUNT`.
- The writable `.` mount — drops to read-only unconditionally (step 6); with no working-tree write, `specify` needs no write route at all.
- Path-math surface of `Locations` (`store_entry`, `store_meta`, `component`, `cache_dir`) replaced by key formulas.
- `bindings.yaml` / `SpecSet.bindings` — sticky argv snapshot folded into the generation digest; nothing reads it back. Adapter list is argv / `sources.toml`; requirement provenance is `Sources:` in `spec.md`.
- `receipts.yaml` / `SpecSet.receipts` / `extract::Receipt` — per-source claim-set digest folded into the generation; nothing reads it back. Extract still runs; the IR flows into reconcile/synthesise and then only survives as `spec.md` / `design.md`.
- `.emery/project.yaml`, `emery_engine::project::Project`, `RequestContext`'s project load, `Error::NotInitialized` / `Error::CliTooOld` as project-file gates, and — under option 1 — the `init` verb, `--upgrade`, and the `/emery:init` skill wrapper ([§5](#5-deleting-projectyaml)).

## 7. Execution steps

Each step is a separate change: journey test green, `cargo make ci` green.

**Prerequisite — delete `project.yaml`.** Lands independently of the storage port; must precede step 6 so `atomic.rs` has no project-config call site left. Preferred shape is §5 option 1 (`specify` takes the adapters positionally or via `--sources sources.toml`; `init` deleted). The journey test becomes a single `specify` over the mock source. Route-budget and `not-initialized` tripwires update in the same change. Option 3 only if the clone question in §5.3 is answered against option 1.

`bindings.yaml` and `receipts.yaml` are deleted from the generation set independently (`SpecSet` is spec + design). `specify` still reads the project record until option 1 lands; it stops snapshotting argv or extract receipts into the generation.

**Step 1 — the storage seam, filesystem-backed.** Introduce the engine storage capability traits (keyvalue + blobstore shapes, §3.3) with a native filesystem implementation that preserves today's on-disk layout byte-for-byte, and route `home.rs`, the cache, and the store through it. Pure refactor: no observable change, no WIT dependency yet. Native tests gain the scripted in-memory storage provider beside the scripted `Model` / `Source`. Update `docs/standards/testing.md` for the boundary shift (engine state is observed through the storage provider and envelope, not the filesystem).

**Step 2 — omnia host bindings.** Wire the `WasiKeyValue` / `WasiBlobstore` host entries into `src/main.rs` and route the step-1 traits over the omnia guest capabilities (`StateStore` / `BlobStore`, §3.3). The engine guest stops opening engine-state paths. The omnia side largely pre-exists — forked WITs, host implementations, guest capabilities, and the `omnia-filesystem` blobstore backend (§3.4 note) — so this step is not externally blocked. The one remaining omnia-side gap is a durable filesystem backing for keyvalue (production backends today are Redis/NATS; the default is in-memory): the shipped local binary needs the `current` pointer to survive restarts, via either a filesystem keyvalue backend in `omnia-backends` or the pointer keyed into the filesystem blobstore binding.

**Step 3 — pointer and generations.** `current` becomes a keyvalue entry; generation sets become blobs keyed by digest; commit order is blobs-then-pointer-swap. Delete the litter half of `prune`; superseded-generation prune on pointer swap stays. The `.emery/spec/` tree stops being written. Migrate locks to keyvalue CAS if lock stamps still exist on the live surface.

**Step 4 — component cache and store move.** Cache and global store entries become blobstore objects; `.meta` provenance moves to keyvalue; verify-on-read digests unchanged. Delete the cache mount, `cache_dir()`, and the mkdir in `main.rs`. Local-`.wasm` mirroring (today `init`'s `ensure`; after §5, `specify`'s) writes through the capability.

**Step 5 — `emery show`.** One read verb over the storage capability (layer 1): `emery show spec|design` of the current generation. Envelope shapes documented in `docs/reference/cli-output-shapes.md`; `cargo make links` gates the doc changes. Route-budget tripwire updates in the same change (`init` already gone from the prerequisite, or this PR names that deletion).

**Step 6 — read-only working tree.** `specify` stops writing `spec.md` / `design.md` into the working tree; the `.` mount drops to read-only and the C3 least-authority posture holds outright. Review is the envelope plus `show`. Update reference docs in the same change.

**Step 7 — read-only MCP resource.** Serve the current generation and id on the existing listener (layer 2). The C3 refusal contract is untouched: mutating routes still refuse. The plugin skill may consume it.

**Step 8 — deployment profiles.** Document and exercise one non-filesystem binding end-to-end (project-id-keyed, multi-project host) to prove the freedom is real. Layer 3 (git projection) is scoped as its own design if wanted.

## 8. Risks and open questions

1. **WIT instability** (§3.4): upstream `wasi-keyvalue` / `wasi-blobstore` churn lands on us as seam maintenance. Mitigated: omnia already vendors and pins its fork of both WITs and owns the host implementations, so churn is absorbed there as a versioned seam change, as with the adapter WIT.
2. **Test surface shift**: the filesystem stops being a public observable boundary for engine state; the scripted storage provider and the envelope become the boundary. Handled in step 1.
3. **Clone story for bindings** ([§5.3](#53-the-gating-question)): option 1 is the default, and the `--sources sources.toml` carrier (§5.2) lets a clone remember the bindings as a committed, operator-owned file — dissolving most of the pressure toward option 3. The residual case is a clone that must `specify` with no argv at all: that is a decision to pick option 3 or implicit `sources.toml` discovery, each of which reintroduces a `project.yaml`-shaped residue (a store key, or a discovered config noun). Do not leave that implicit.
4. **Performance of remote bindings**: unconfirmed; measure in step 8 before claiming anything.
