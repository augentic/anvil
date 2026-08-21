# Portable storage and the human seam

Status: **Draft** — working design, executed as the discrete steps in [§6](#6-execution-steps). Each step lands independently with the journey test green.

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
4. Net deletion where the backing permits it: `atomic.rs`, the litter half of `prune`, the `cache_dir()` mkdir in `main.rs`, and the writable `.` mount all go.

Non-goals:

- Moving the **workspace lend**. The WIT `workspace` record (`wit/emery.wit`) lends the operator's live source tree read-only to the model; it is inherently a filesystem view and stays one. (A content-addressed workspace snapshot for reproducible extraction is a separate, larger design.)
- Dynamic adapter resolution or any download path.
- A migration framework. Pre-1.0, crossing this boundary is a re-init.

## 3. Target architecture

### 3.1 Storage inventory and destination

| Surface | Today | Destination |
| --- | --- | --- |
| Generation documents (`bindings.yaml`, `receipts.yaml`, `spec.md`, `design.md`) | `.emery/spec/generations/<digest>/` | blobstore container, objects keyed by generation digest |
| `current` pointer | `.emery/spec/current` file | keyvalue entry; swap is one atomic `set` |
| Component cache | `.emery-cache/components/<name>.wasm` preopen | blobstore container keyed by adapter name |
| Global adapter store + `.meta` sidecars | host-side `<store>/<name>@<version>.wasm` + sidecar | blobstore (immutable entries) + keyvalue (provenance sidecars) |
| Locks / PID stamps | file stamps via `bytes_write` | keyvalue atomics (CAS / increment) |
| `project.yaml` | `.emery/project.yaml` | **stays on disk** — operator-reviewable, travels with the repo ([§4](#4-the-human-seam)) |
| Workspace lend | read-only view of the source tree | **stays on disk** (non-goal) |

### 3.2 Authority model

- The **keyvalue pointer is the single authority** for "what is the current generation". A read failure on the pointer is an error, never an empty result — exactly the posture `Home::current` already takes with `spec-home-corrupt`.
- Generation blobs are **immutable and self-verifying**: `SpecSet::id()` is the digest of the documents' bytes, so any copy anywhere can be checked against the pointer. Projections ([§4](#4-the-human-seam)) are therefore never trusted, only verified.
- Multi-document commit needs no transaction: write the immutable blobs first, then swap the one pointer key — the ordering `Home::commit` already implements over the filesystem.

### 3.3 The capability seam

Follow the existing provider pattern (`omnia_guest::Model`, `emery_adapter::SourceDispatch`; see the bare `Provider` in `src/lib.rs`): one engine-side storage capability trait pair, with wasm32 defaults over the `wasi:keyvalue` / `wasi:blobstore` imports and bare native impls so tests script storage in memory exactly as they script the model and the source seam today.

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

`project.yaml` follows the same logic and simply stays on disk: it is the operator-facing binding record and belongs in the repo.

### Layer 3 — read-only MCP resource

The pre-bound listener already serves MCP reference shelves only, with the typed C3 refusal (`crates/transport/src/http.rs`) rejecting everything else. A read-only resource exposing the current generation and its id fits that posture — reads were never what C3 fences — and serves the growing consumer that is not a human at a shell: IDEs and agents (the `plugins/emery/` skill included).

### Layer 4 — git projection

For team and hosted deployments, materialization targets a git ref or a PR instead of the working tree; review becomes code review. The heavier variant — a blobstore host binding backed by a git object database — additionally yields generation history for free. Deferred; recorded here so the container/key naming in step 3 does not preclude it.

### Retention

Today `Home::prune` deletes the superseded generation immediately, which is why the re-mine diff must be computed at commit time. With blob retention this constraint disappears. Default proposal: **retain superseded generations**, pruned by a configurable policy (count- or age-bounded; host-binding default keeps a small bounded history). This is what makes `emery diff <from> <to>` and layer-4 history possible. Growth numbers: unconfirmed — measure before choosing the default bound.

## 5. Deletions

- `crates/artifacts/src/atomic.rs` — blobstore writes are complete-on-finalize; the keyvalue `set` is atomic. (Survives only as long as `project.yaml` and materialized projections still need a crash-safe disk write; deleted or reduced to those call sites.)
- The crash-litter half of `Home::prune` — no temp files exist to leak; retained-generation pruning becomes key enumeration under the retention policy.
- `cache_dir()` and its `create_dir_all` in `src/main.rs`; the cache mount and `GUEST_CACHE_MOUNT`.
- The writable `.` mount — drops to read-only once materialization's write route is settled (step 6).
- Path-math surface of `Locations` (`store_entry`, `store_meta`, `component`, `cache_dir`) replaced by key formulas.
- The commit-time-only diff constraint in `Home::outgoing` — subsumed by `emery diff` over retained generations.

## 6. Execution steps

Each step is a separate change: journey test green, `cargo make ci` green.

**Step 1 — the storage seam, filesystem-backed.** Introduce the engine storage capability traits (keyvalue + blobstore shapes, §3.3) with a native filesystem implementation that preserves today's on-disk layout byte-for-byte, and route `home.rs`, the cache, and the store through it. Pure refactor: no observable change, no WIT dependency yet. Native tests gain the scripted in-memory storage provider beside the scripted `Model` / `SourceDispatch`. Update `docs/standards/testing.md` for the boundary shift (engine state is observed through the storage provider and envelope, not the filesystem).

**Step 2 — omnia host bindings.** Land `WasiKeyvalue` / `WasiBlobstore` hosts in omnia with the filesystem-backed default; add the wasm32 default impls of the step-1 traits over the WIT imports; add the host entries in `src/main.rs`. The engine guest stops opening engine-state paths. Prerequisite owned outside this repo; this step is blocked, not partial, until the omnia side exists.

**Step 3 — pointer, generations, retention.** `current` becomes a keyvalue entry; generation sets become blobs keyed by digest; commit order is blobs-then-pointer-swap; retention policy per §4. Delete the litter half of `prune`. The `.emery/spec/` tree stops being written. Migrate locks to keyvalue CAS if lock stamps still exist on the live surface.

**Step 4 — component cache and store move.** Cache and global store entries become blobstore objects; `.meta` provenance moves to keyvalue; verify-on-read digests unchanged. Delete the cache mount, `cache_dir()`, and the mkdir in `main.rs`. `emery init`'s local-`.wasm` mirroring writes through the capability.

**Step 5 — read verbs.** `emery show` and `emery diff` over the storage capability (layer 1). `emery diff` reads retained generations; the commit-time diff in the `specify` envelope stays for continuity but is now a convenience, not the only window. Envelope shapes documented in `docs/reference/cli-output-shapes.md`; `cargo make links` gates the doc changes.

**Step 6 — materialization as checkout.** `specify` materializes `spec.md` / `design.md` (+ generation-id stamp) into the working tree as a projection; `emery materialize` re-projects on demand. Decide the write route (host-side vs narrow guest write capability) and drop the `.` mount to read-only. Update reference docs in the same change.

**Step 7 — read-only MCP resource.** Serve the current generation and id on the existing listener (layer 3). The C3 refusal contract is untouched: mutating routes still refuse. The plugin skill may consume it.

**Step 8 — deployment profiles.** Document and exercise one non-filesystem binding end-to-end (project-id-keyed, multi-project host) to prove the freedom is real. Layer 4 (git projection) is scoped as its own design if wanted.

## 7. Risks and open questions

1. **WIT instability** (§3.4): upstream `wasi-keyvalue` / `wasi-blobstore` churn lands on us as seam maintenance. Mitigation: vendor and pin, as with the adapter WIT.
2. **Materialization write route** (step 6): host-side write keeps the guest fully read-only on `.` but moves a product behaviour into deployment policy; a narrow guest write capability keeps the behaviour in the engine at the cost of one more capability. Decide in step 6 with the C3 posture in the room.
3. **Test surface shift**: the filesystem stops being a public observable boundary for engine state; the scripted storage provider and the envelope become the boundary. Handled in step 1.
4. **`project.yaml` residue**: it remains an engine-written disk file alongside materialized projections. If step 6 lands the host-side write route, `atomic.rs` shrinks to those call sites — decide then whether they move too or disk residence is affirmed as product policy.
5. **Retention bound** (§4): the default history bound is unmeasured; pick after observing generation sizes in step 3.
6. **Performance of remote bindings**: unconfirmed; measure in step 8 before claiming anything.
