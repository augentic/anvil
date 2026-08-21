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
2. The host binding chooses the backing; the shipped local binary keeps filesystem-backed behaviour indistinguishable to the operator.
3. The human seam — review, diff, git-tracking of `spec.md` / `design.md` — survives as **verifiable, non-authoritative projections** of the store, with first-class read verbs ([§4](#4-the-human-seam)).
4. Net deletion where the backing permits it: `atomic.rs`, the litter half of `prune`, the `cache_dir()` mkdir in `main.rs`, the writable `.` mount, and `.emery/project.yaml` ([§5](#5-deleting-projectyaml)) all go.

Non-goals:

- Moving the **workspace lend**. The WIT `workspace` record (`wit/emery.wit`) lends the operator's live source tree read-only to the model; it is inherently a filesystem view and stays one. (A content-addressed workspace snapshot for reproducible extraction is a separate, larger design.)
- Dynamic adapter resolution or any download path.
- A migration framework. Pre-1.0, crossing this boundary is a re-init.
- A human-edited replacement config (`emery.yaml` at the repo root, etc.). That reverses the rule that the CLI is the only mutation path and recreates the same noun.

## 3. Target architecture

### 3.1 Storage inventory and destination

| Surface | Today | Destination |
| --- | --- | --- |
| Generation documents (`bindings.yaml`, `receipts.yaml`, `spec.md`, `design.md`) | `.emery/spec/generations/<digest>/` | blobstore container, objects keyed by generation digest |
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

`emery_engine::home` remains the one module owning spec-set reads/writes; `Locations` stops being path math and becomes key/container-name math. Kernels keep consuming values and never touch the environment.

### 3.4 Host bindings

The `omnia::runtime!` invocation in `src/main.rs` grows `WasiKeyvalue` / `WasiBlobstore` host entries beside `WasiHttp` / `WasiModel`, and the `mounts:` table shrinks (cache mount deleted; `.` drops to read-only once materialization is a projection). The shipped binary's default binding is filesystem-backed under the invocation directory so local behaviour is unchanged; alternative bindings (project-id-keyed, remote) are deployment profiles, not engine changes.

> [!WARNING]
> `wasi-keyvalue` and `wasi-blobstore` are early-phase WASI proposals: WITs are unstable and stock wasmtime does not ship host implementations. The omnia host bindings are a prerequisite owned in the omnia repo (step 2 gate). Pin the WIT versions we vendor and treat upstream churn as a versioned seam change, like the adapter WIT.

## 4. The human seam

Two standing facts shrink this problem:

1. **The filesystem is already read-only to humans.** The CLI contract forbids hand-editing anything under `.emery/`; every mutation routes through the CLI. The seam to preserve is *review* — read, diff, track — never edit.
2. **Every generation is self-verifying** (§3.2), so we can hand out any number of non-authoritative views without forking authority.

The seam is delivered in layers over one authority. Layers 1–2 are part of this work's definition of done; layers 3–4 are follow-on deployment profiles.

### Layer 0 — the envelope (exists)

The `specify` envelope is already the reporting channel: the re-mine diff is emitted, never persisted. All new read paths follow that precedent — rendered from the store, emitted, never a second authority.

### Layer 1 — read verbs

The CLI grows a first-class read surface over the store:

- `emery show spec|design|bindings|receipts [--generation <id>]` — render a document from the store to stdout, with the generation id in the envelope.
- `emery diff [<from> [<to>]]` — the re-mine diff on demand between any two retained generations, defaulting to previous-vs-current. This subsumes the commit-time-only diff: once superseded generations are retained (§retention), the diff stops being a one-shot side effect of `specify` and becomes a plain read.
- `emery materialize [--dir <path>]` — explicitly write the current generation's documents into the working tree ([layer 2](#layer-2--materialization-as-checkout)).

These are pure reads over the storage capability and work identically in every deployment, filesystem or not.

### Layer 2 — materialization as checkout

`specify` keeps writing `spec.md` / `design.md` into the working tree by default (with `emery materialize` as the explicit re-projection), but **demoted to a projection**: a rendered copy of the committed generation, stamped with the generation id, rewritten on each commit, verifiable against the pointer. This preserves the load-bearing product property that *the spec is in the repo when you clone it* — the operator commits the documents like any generated file. A torn or stale projection is harmless: the store is the authority and the digest exposes drift.

Bindings are not a third projection. The generation already snapshots them as `bindings.yaml` (a receipt folded into the digest). The live input that today's `project.yaml` holds is deleted with the file ([§5](#5-deleting-projectyaml)) — not re-homed onto disk under another name.

### Layer 3 — read-only MCP resource

The pre-bound listener already serves MCP reference shelves only, with the typed C3 refusal (`crates/transport/src/http.rs`) rejecting everything else. A read-only resource exposing the current generation and its id fits that posture — reads were never what C3 fences — and serves the growing consumer that is not a human at a shell: IDEs and agents (the `plugins/emery/` skill included).

### Layer 4 — git projection

For team and hosted deployments, materialization targets a git ref or a PR instead of the working tree; review becomes code review. The heavier variant — a blobstore host binding backed by a git object database — additionally yields generation history for free. Deferred; recorded here so the container/key naming in step 3 does not preclude it.

### Retention

Today `Home::prune` deletes the superseded generation immediately, which is why the re-mine diff must be computed at commit time. With blob retention this constraint disappears. Default proposal: **retain superseded generations**, pruned by a configurable policy (count- or age-bounded; host-binding default keeps a small bounded history). This is what makes `emery diff <from> <to>` and layer-4 history possible. Growth numbers: unconfirmed — measure before choosing the default bound.

## 5. Deleting `project.yaml`

`.emery/project.yaml` is the spec generator's authored project record: identity, the CLI version pin, and the source bindings `specify` extracts from. Written only by `emery init`; loaded fail-closed by `specify` via `RequestContext`. Humans must not edit it.

It is not a twin of the generation store. The store is output (`spec.md`, `design.md`, receipts, a frozen copy of the bindings). `project.yaml` is a **sticky copy of `init`'s argv** — which adapters, under which key, workspace vs `--value`. The generation's `bindings.yaml` is a receipt of that list for that run, not something later runs read back.

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

Three ways to keep the operator journey without the file. Options 2 and 3 keep the *concept* and delete only the YAML path. Option 1 removes the concept.

**Option 1 — collapse `init` into `specify` (preferred).** `emery specify <adapter>... [--value <adapter>=<text>]`. Ensure, extract, commit in one verb.

- Persistence of the binding list goes away. Repeat the adapters every run (Makefile, skill, CI).
- `not-initialized` dies. There is no project record to be missing.
- `--upgrade` dies. Re-running `specify` re-ensures as a side effect of resolve, the way extract already re-resolves.
- Local `.wasm` mirroring moves to the first `specify` (today that is `ensure` inside `init`).
- Generation `bindings.yaml` remains the snapshot of *this* run's argv.
- Changing sources is a different argv. Intent text (`--value`) is passed every time unless the operator points at a file they own.
- Constitution impact: invariant 2's live route budget shrinks from `init` + `specify` to `specify`. That is a policy change in the same PR (invariant 5). The `/emery:init` skill wraps the surviving verb or is deleted with it.

This is the deletion-aligned option: one verb, no project-config noun, no sticky file. Cost: the adapter list is no longer remembered inside the tree.

**Option 2 — the current generation is the binding authority.** First run takes adapters: `emery specify <adapter>...`. Later runs with no adapters reuse `current`'s `bindings.yaml`.

- Initialized = the `current` pointer exists (fail-closed, same posture `Home::current` already has).
- `--upgrade` reads that snapshot and re-ensures those selectors.
- Changing bindings needs an explicit rewrite. Silent reuse of a stale list is the hazard.
- `init` shrinks to "cache + first binding write" or disappears into first `specify`.
- Fits the store: bindings are already a generation document; the pointer is already the one authority.
- Cost: you cannot record bindings without producing a spec. Today `init` may bind without extracting. A first `specify` that fails the extras gate leaves no committed bindings unless a bindings-only generation (a new kind) or a store key for "authored but never generated" exists — both are adds.
- Clone story: if generations stay in `.emery/spec/` and get committed, clone works. If they move to blobstore (step 3), clone only works if `bindings.yaml` is materialized as a checkout — a file again, just not `project.yaml`.

**Option 3 — bindings live in the store, not on disk.** `init` writes a keyvalue entry (selectors + values). `specify` reads it. The YAML file is gone; the data is not.

- Initialized = that key exists.
- `--upgrade` re-ensures from the key.
- Step 1 already wants this shape for engine state.
- Clone/CI still need a host binding that survives `git clone` (filesystem-backed store under the repo, or a materialized projection). Otherwise a fresh checkout cannot `specify` without re-passing adapters — which collapses to option 1.

This is a rename of `project.yaml` into the store. Worth it only if the store is coming anyway and a git-tracked file is refused, but the clone story still demands a projection.

**Rejected:** a human-edited `emery.yaml` at the repo root. Same noun, inverted mutation rule.

### 5.3 The gating question

After `git clone`, how does `specify` know the adapters?

- Remembered in argv / CI → option 1, and `init` can go.
- Remembered in the last generation → option 2, and that generation must be materialized or committed.
- Remembered in host storage → option 3, and the local binary's filesystem binding has to look like today's `.emery/` to the operator.

Option 1 plus adapter-floor-only versioning is the one that does not leave a `project.yaml`-shaped residue for step 6 and `atomic.rs` to worry about. Options 2 and 3 are recorded so a clone-must-remember-bindings requirement can still pick them; they are not the default.

## 6. Deletions

- `crates/artifacts/src/atomic.rs` — blobstore writes are complete-on-finalize; the keyvalue `set` is atomic. (Survives only as long as materialized projections still need a crash-safe disk write; deleted or reduced to those call sites. `project.yaml` is not one of them.)
- The crash-litter half of `Home::prune` — no temp files exist to leak; retained-generation pruning becomes key enumeration under the retention policy.
- `cache_dir()` and its `create_dir_all` in `src/main.rs`; the cache mount and `GUEST_CACHE_MOUNT`.
- The writable `.` mount — drops to read-only once materialization's write route is settled (step 6).
- Path-math surface of `Locations` (`store_entry`, `store_meta`, `component`, `cache_dir`) replaced by key formulas.
- The commit-time-only diff constraint in `Home::outgoing` — subsumed by `emery diff` over retained generations.
- `.emery/project.yaml`, `emery_engine::project::Project`, `RequestContext`'s project load, `Error::NotInitialized` / `Error::CliTooOld` as project-file gates, and — under option 1 — the `init` verb, `--upgrade`, and the `/emery:init` skill wrapper ([§5](#5-deleting-projectyaml)).

## 7. Execution steps

Each step is a separate change: journey test green, `cargo make ci` green.

**Prerequisite — delete `project.yaml`.** Lands independently of the storage port; must precede step 6 so `atomic.rs` has no project-config call site left. Preferred shape is §5 option 1 (`specify` takes the adapters; `init` deleted). The journey test becomes a single `specify` over the mock source. Route-budget and `not-initialized` tripwires update in the same change. Options 2/3 only if the clone question in §5.3 is answered against option 1.

**Step 1 — the storage seam, filesystem-backed.** Introduce the engine storage capability traits (keyvalue + blobstore shapes, §3.3) with a native filesystem implementation that preserves today's on-disk layout byte-for-byte, and route `home.rs`, the cache, and the store through it. Pure refactor: no observable change, no WIT dependency yet. Native tests gain the scripted in-memory storage provider beside the scripted `Model` / `Source`. Update `docs/standards/testing.md` for the boundary shift (engine state is observed through the storage provider and envelope, not the filesystem).

**Step 2 — omnia host bindings.** Land `WasiKeyvalue` / `WasiBlobstore` hosts in omnia with the filesystem-backed default; add the wasm32 default impls of the step-1 traits over the WIT imports; add the host entries in `src/main.rs`. The engine guest stops opening engine-state paths. Prerequisite owned outside this repo; this step is blocked, not partial, until the omnia side exists.

**Step 3 — pointer, generations, retention.** `current` becomes a keyvalue entry; generation sets become blobs keyed by digest; commit order is blobs-then-pointer-swap; retention policy per §4. Delete the litter half of `prune`. The `.emery/spec/` tree stops being written. Migrate locks to keyvalue CAS if lock stamps still exist on the live surface.

**Step 4 — component cache and store move.** Cache and global store entries become blobstore objects; `.meta` provenance moves to keyvalue; verify-on-read digests unchanged. Delete the cache mount, `cache_dir()`, and the mkdir in `main.rs`. Local-`.wasm` mirroring (today `init`'s `ensure`; after §5, `specify`'s) writes through the capability.

**Step 5 — read verbs.** `emery show` and `emery diff` over the storage capability (layer 1). `emery diff` reads retained generations; the commit-time diff in the `specify` envelope stays for continuity but is now a convenience, not the only window. Envelope shapes documented in `docs/reference/cli-output-shapes.md`; `cargo make links` gates the doc changes.

**Step 6 — materialization as checkout.** `specify` materializes `spec.md` / `design.md` (+ generation-id stamp) into the working tree as a projection; `emery materialize` re-projects on demand. Decide the write route (host-side vs narrow guest write capability) and drop the `.` mount to read-only. Update reference docs in the same change.

**Step 7 — read-only MCP resource.** Serve the current generation and id on the existing listener (layer 3). The C3 refusal contract is untouched: mutating routes still refuse. The plugin skill may consume it.

**Step 8 — deployment profiles.** Document and exercise one non-filesystem binding end-to-end (project-id-keyed, multi-project host) to prove the freedom is real. Layer 4 (git projection) is scoped as its own design if wanted.

## 8. Risks and open questions

1. **WIT instability** (§3.4): upstream `wasi-keyvalue` / `wasi-blobstore` churn lands on us as seam maintenance. Mitigation: vendor and pin, as with the adapter WIT.
2. **Materialization write route** (step 6): host-side write keeps the guest fully read-only on `.` but moves a product behaviour into deployment policy; a narrow guest write capability keeps the behaviour in the engine at the cost of one more capability. Decide in step 6 with the C3 posture in the room.
3. **Test surface shift**: the filesystem stops being a public observable boundary for engine state; the scripted storage provider and the envelope become the boundary. Handled in step 1.
4. **Clone story for bindings** ([§5.3](#53-the-gating-question)): option 1 is the default (argv / CI remember the adapters). If a later requirement is that a clone must `specify` with no argv, that is a decision to pick option 2 or 3 — and it reintroduces a `project.yaml`-shaped residue (materialized `bindings.yaml` or a store key). Do not leave that implicit.
5. **Retention bound** (§4): the default history bound is unmeasured; pick after observing generation sizes in step 3.
6. **Performance of remote bindings**: unconfirmed; measure in step 8 before claiming anything.
