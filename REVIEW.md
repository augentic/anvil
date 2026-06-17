# RFC-48 / RFC-49 Workspace Review

> Scope: `augentic/specify` (prose), `augentic/specify-cli` (engine), `augentic/specify-adapters` (extracted adapters), reviewed against [`rfcs/rfc-48-adapter-packaging-transport.md`](rfcs/rfc-48-adapter-packaging-transport.md), the implementation plan (`rfc-48_adapter_packaging`), and [`rfcs/rfc-49-repository-topology.md`](rfcs/rfc-49-repository-topology.md).
> Branches reviewed: `specify@rfc-48`, `specify-cli@rfc-48`, `specify-adapters@main`.
> Method: static reading of the implementation, three focused code-trace passes, and live execution of `make lint` (prose), `specify lint framework`, and `specify adapter build` against the extracted adapters using a freshly built `specify-cli@rfc-48` binary.

## Executive summary

The rename (`tool` → `extension`/`registry`), the D11 manifest reshape, the byte-deterministic pack, the OCI publish loop, and the D5 store primitives are **individually well-built and tested**. But the work has been integrated and "moved" without ever running the end-to-end path it exists to enable, and that gap hides one **showstopper** plus several blocking defects:

- **The two extracted adapters cannot be built or published at all.** The D12 shared-content fork was never actually performed — the `adapters/shared/references/runtime/**` symlinks in `specify-adapters` still point at the platform repo's `plugins/`/`docs/`, so they dangle, and `specify adapter build` aborts on the first one. This is the core RFC-48 deliverable and it is non-functional.
- **The adapters repo cannot lint itself.** `specify lint framework` hard-requires a `plugins/` dir, which the adapters repo does not have, so the RFC-49 T5 `framework-lint` CI job is dead-on-arrival.
- **D4 verify-on-read is absent.** The digest is verified only at publish; it is never recorded at install nor checked on read. The "integrity lives upstream, downstream is a one-line verify" guarantee is not wired, and the code that would do it (`store::install`) is dead.
- **The default operator path never touches the new transport.** `specify init omnia@1.0.0` still uses git sparse-checkout + the per-project manifest cache; only the `specify:<name>@<semver>` package-ref form reaches the store.

Net: the *primitives* are landed; the *system* is not yet wired or verified end-to-end. Several defects below would each fail CI or fail at first real use.

### Severity index

| # | Severity | Finding | Repo(s) |
| --- | --- | --- | --- |
| [C1](#c1) | **Critical** | Extracted adapters unbuildable/unpublishable — D12 shared fork never done (dangling symlinks) | specify-adapters |
| [H1](#h1) | High | `specify lint framework` rejects the adapters repo (`plugins/` required) → T5 CI job always red | specify-cli, specify-adapters |
| [H2](#h2) | High | D4 verify-on-read unimplemented; `store::install` (digest path) is dead code | specify-cli |
| [H3](#h3) | High | Default `init <name>@<ver>` bypasses registry/store; transport unexercised end-to-end | specify-cli |
| [M1](#m1) | Medium | Contradictory tests: `adapter` verb asserted both retired and active → CI red | specify-cli |
| [M2](#m2) | Medium | Registry basic-auth env var drift (`SPECIFY_REGISTRY_USER` vs `…_USERNAME`) | specify-cli, specify-adapters |
| [M3](#m3) | Medium | Forked framework rules drifted: stale `CORE-049` in adapters; required `adapter-extension-crate-missing` never implemented | specify-cli, specify-adapters |
| [M4](#m4) | Medium | User-facing docs still teach the retired `tools[]`/`tools.yaml` model | specify |
| [L1](#l1) | Low | `AGENTS.md` crate-graph still names `specify-tool*` (pre-rename) | specify-cli |
| [L2](#l2) | Low | Vestigial `use-local-dev.rs` WASI build + `Makefile` prose (RFC-49 T4 cleanup) | specify |
| [L3](#l3) | Low | Per-project manifest-cache copy still runs on the store path (D5 "resolve in place") | specify-cli |
| [L4](#l4) | Low | Uncommitted `.gitignore` change drops ignores (`.specify/…`, `stderr.log`, `/specify`) | specify-cli |
| [L5](#l5) | Low | Forked `shared/` already drifting at byte level (`rules/universal/README.md`) | specify-adapters |
| [O1–O5](#opportunities) | Opt | Single-layer OCI, per-call tokio runtime, blocking lock, `Store`/`Cached` duplication, committed 6.5 MB wasm | specify-cli, specify-adapters |

---

## Critical

### C1 — The extracted adapters cannot be built or published; the D12 shared-content fork was never performed {#c1}

This is the headline defect. RFC-48 D12 (and the phasing step-8 "one-time fork") require that, when adapters relocate, the adapter-needed subset of `plugins/spec/references/` + `docs/reference/` is **clean-copied** into the adapters repo so the `adapters/shared/` symlink targets resolve in-repo. That copy did not happen. Instead the symlinks were relocated verbatim and now point outside the repo:

```text
specify-adapters/adapters/shared/references/runtime/guardrails.md
  -> ../../../../plugins/spec/references/guardrails.md      [DANGLING — no plugins/ in this repo]
specify-adapters/adapters/shared/references/runtime/review-team-protocol.md
  -> ../../../../docs/reference/review-team-protocol.md      [DANGLING — no docs/ in this repo]
specify-adapters/adapters/targets/vectis/references/agent-teams.md
  -> ../../../shared/references/runtime/review-team-protocol.md  [DANGLING via the above]
```

All 17 files under `adapters/shared/references/runtime/**` are dangling, and the RFC explicitly predicted exactly this failure mode: *"relocating `adapters/` before clean-copying that subset … either dangles those symlinks or drags spec-plugin prose into the wrong repo."*

Because `pack_adapter` resolves entries with `std::fs::metadata` (follows symlinks, errors on a missing target), the build/publish path aborts. Proven live with the `rfc-48` binary:

```console
$ specify adapter build --path adapters/targets/contracts --dry-run
error: adapter-pack-failed: adapter pack failed: stat adapters/targets/contracts/references/spec-runtime/guardrails.md: No such file or directory (os error 2)

$ specify adapter build --path adapters/targets/vectis --dry-run
error: adapter-pack-failed: adapter pack failed: stat adapters/targets/vectis/references/spec-runtime/guardrails.md: No such file or directory (os error 2)
```

**Impact.** Both extension-bearing adapters (`contracts`, `vectis`) are unbuildable and unpublishable. The `specify-adapters/.github/workflows/release.yaml` publish job (`specify adapter build … && specify adapter publish …`) fails on the first adapter. RFC-48's entire reason for moving these adapters — self-contained, publishable artifacts — is unmet. This was missed because the plan itself flagged publish as *"Unverifiable here: needs the repo pushed + registry secrets + a live run,"* so the pack step was never exercised; `cargo check`/`clippy` green only proves the Rust crates compile, not that the prose tree packs.

**Why it matters beyond pack.** Even if pack tolerated dangling links, the published artifact would be missing the `spec` bundle — a D3 self-containment violation. And the framework `§F1` walk also follows symlinks, so lint would trip on the same dangling targets.

**Recommended fix.** Perform the D12 fork for real: dereference (clean-copy) the adapter-needed subset of `plugins/spec/references/**` + `docs/reference/**` into `specify-adapters` so that `adapters/shared/references/runtime/**` (and the `agent-teams.md` target) are **real files**, and re-point the hub at the in-repo copy. Then add a guard so this cannot regress silently — e.g. a `specify adapter build --dry-run` smoke step in the adapters `ci.yaml` (it needs no registry, no secrets, and no wasm toolchain), and/or have `pack_adapter` raise a precise `adapter-pack-symlink-dangling` finding naming the unresolved link rather than a bare `os error 2`.

---

## High

### H1 — `specify lint framework` rejects the adapters repo, so the RFC-49 T5 CI seam is dead-on-arrival {#h1}

`canonical_framework_root` requires **both** `plugins/` and `adapters/`:

```rust
// specify-cli/src/runtime/commands/lint/framework.rs:122
fn canonical_framework_root(root: &Path) -> Result<PathBuf> {
    if !(root.join("plugins").is_dir() && root.join("adapters").is_dir()) {
        return Err(Error::Diag { code: "framework-root",
            detail: format!("not a framework root: {}", root.display()) });
    }
    ...
}
```

`specify-adapters` has `adapters/` but no `plugins/`. Proven live:

```console
$ specify lint framework --framework-root .   # in specify-adapters
error: framework-root: not a framework root: .
```

The adapters `ci.yaml` `framework-lint` job (the job RFC-49 T5 claims is "already shipped … explicitly tagged RFC-49 T5") runs exactly `specify lint framework --framework-root .`, so it will always exit non-zero. RFC-49's assertion that the adapters→platform seam is functional is therefore **incorrect** as built.

**Deeper issue.** The framework profile is plugin-centric (marketplace, skill-body, links-registry, etc.). An adapters-only tree needs an "adapters subset" of checks. Relaxing the root predicate to accept `adapters/` alone is necessary but not sufficient — the indexer/marketplace checks must tolerate the absence of `plugins/` and `.cursor-plugin/marketplace.json`.

**Recommended fix.** Introduce an adapters-aware framework root (accept `adapters/` with or without `plugins/`) and scope the plugin/marketplace checks to "run only when `plugins/` is present," so the adapters repo runs the adapter-manifest/brief/rules checks and skips the skill/marketplace family. Add an integration test that lints a synthetic `adapters/`-only root. (This pairs naturally with the fix for [C1](#c1) and [M3](#m3), since lint over the adapters repo would also catch the dangling symlinks and the stale `CORE-049`.)

### H2 — D4 verify-on-read is not wired; the digest path is dead code {#h2}

D4 is the RFC's trust anchor: *"The consumer records [the digest] on install and re-checks it on every read, refusing a mismatch."* What is actually implemented:

- The digest is computed and verified **only at publish** (pull-back round-trip): `src/runtime/commands/adapter.rs` `publish()`.
- `store::install_tofu` (the only install path reached at runtime) is **trust-on-first-use**: it pulls and unpacks with **no recorded digest and no verify**.
- `store::install(name, version, reference, recorded_digest, auth)` — the function that *would* verify on install — has **zero call sites** in either repo.
- No install-metadata file is written, and **nothing re-hashes a store entry on read** (`SourceAdapter::resolve` / `TargetAdapter::resolve` / `locate_axis` never call `verify_digest`).
- The `adapter-digest-mismatch` finding exists but is reachable only through the dead `install` path and the publish round-trip.

This is partly acknowledged (`AGENTS.md`: *"recorded-digest `project.yaml` pin for cross-machine verify-on-read (D4) deferred to a follow-up"*), but the RFC lists D4 in the v1 test plan, so it is a conformance gap, not just a future nicety. As-is, a corrupted or tampered store entry is never detected on read.

**Recommended fix.** Either (a) finish D4: record the registry content digest in store-entry install metadata at `install_tofu` time and re-verify on each `Store` read; or (b) if intentionally deferring, delete `store::install` (currently dead) and state the deferral explicitly in `DECISIONS.md` with the finding code marked "publish-only in v1," so the dead code does not masquerade as a wired guarantee.

### H3 — The default first-party init path bypasses the registry and the store entirely {#h3}

RFC-48's CLI surface says `specify init omnia@1.0.0` *"pulls the published artifact once, installs into the shared store."* In practice:

- `recognize_package("omnia@1.0.0")` returns `None` (package refs require the `specify:` namespace prefix), so `install_adapter_package` is a no-op.
- `init` then resolves the **first-party shorthand** through `from_shorthand` → a GitHub **git sparse checkout** (`init/git.rs`), and copies the result into the **per-project manifest cache** (`init/cache.rs`).
- The global store is only *probed* on later resolve; it is never *populated* on this path, so resolution falls to `AdapterLocation::Cached`.

Only `specify init specify:omnia@1.0.0` reaches `store::install_tofu` → OCI pull → store. Combined with [C1](#c1) (publish is broken) and [H2](#h2) (no verify-on-read), the registry transport is **not exercised end-to-end anywhere**: nothing can be published, and the documented default consumer command does not pull from the registry.

The code comments mark this transitional, which is fair — but the RFC and the CLI-surface docs present it as the working default, so the gap should be tracked explicitly rather than read as "done."

**Recommended fix.** Decide and document the v1 contract: either route first-party shorthand through the package-ref/store path (closing the loop, dependent on [C1](#c1)), or update RFC-48's CLI-surface section + `init` help to state that shorthand remains git-based in this milestone and the registry path is reached only via `specify:<name>@<ver>`.

---

## Medium

### M1 — Contradictory tests over the `adapter` verb will fail CI {#m1}

RFC-48 un-retires `specify adapter` as the packaging group. `tests/target.rs` was updated to assert it is active:

```rust
// specify-cli/tests/target.rs
fn adapter_group_exposes_build_and_publish() { /* asserts build + publish present */ }
```

But `tests/slice/metadata.rs` still asserts it is **retired**:

```rust
// specify-cli/tests/slice/metadata.rs:56
for retired in ["change", "adapter"] {
    assert!(!verbs.iter().any(|v| v == retired),
        "retired verb `{retired}` must not resurface: {verbs:?}");
}
```

The contract dump confirms `adapter` is now a real top-level verb, so `help_lists_axis_verbs` fails. `cargo make ci` (the gate the plan claims green) would go red here.

**Recommended fix.** Drop `"adapter"` from the `retired` set in `tests/slice/metadata.rs` (keep `"change"`).

### M2 — Registry basic-auth env var name drift {#m2}

The binary reads `SPECIFY_REGISTRY_USER`:

```rust
// specify-cli/crates/registry/src/oci.rs:71
match (non_empty_env("SPECIFY_REGISTRY_USER"), non_empty_env("SPECIFY_REGISTRY_PASSWORD")) { ... }
```

But the adapters repo wires and documents `SPECIFY_REGISTRY_USERNAME`:

- `specify-adapters/.github/workflows/release.yaml:59` → `SPECIFY_REGISTRY_USERNAME: ${{ secrets.SPECIFY_REGISTRY_USERNAME }}`
- `specify-adapters/README.md:45` → "`SPECIFY_REGISTRY_USERNAME` / …`_PASSWORD` (basic)"

A basic-auth deploy that sets `SPECIFY_REGISTRY_USERNAME` is silently ignored and falls through to `RegistryAuth::Anonymous`, so an authenticated push fails (or unexpectedly attempts anonymous). It only "works" if a `SPECIFY_REGISTRY_TOKEN` happens to also be set. This is latent until the first credentialed publish — which, given [C1](#c1), has not run.

**Recommended fix.** Pick one name and converge all three sites. `…_USERNAME` is the clearer choice; update `oci.rs` (and its doc-comment at line 64) to read `SPECIFY_REGISTRY_USERNAME`.

### M3 — Forked framework rules have drifted; the RFC-required replacement rule is missing {#m3}

Two related problems from the D12 fork of `adapters/shared/rules/`:

1. **Stale rule survives in the fork.** `specify` correctly retired `CORE-049-tools-invalid-declaration.md` (D11 removes the `adapter-tool` cross-reference rule), but `specify-adapters/adapters/shared/rules/core/CORE-049-tools-invalid-declaration.md` still exists. Its hint is `kind: cross-reference, config: { target: adapter-tool }` with pinned `tools[]` versions (`contracts/contract: 0.3.0`, `vectis/vectis: 0.4.0`) — the retired pre-D11 model. The cross-reference evaluator only supports `adapter-manifest` (`adapter-tool` → `HintError::Unsupported`), so once [H1](#h1) is fixed and lint runs on the adapters repo, this rule will **abort the run**.
2. **Required replacement never implemented.** RFC-48's finding table mandates `adapter-extension-crate-missing` ("replaces the retired `adapter-tool` cross-reference rule"). It appears **only in the RFC** — no CORE rule, no lint facts, no evaluator support anywhere. Consequently, an adapter that declares `extension:` but has deleted its `extension/` crate (while keeping a stale committed `adapter.wasm`) is never flagged.

**Recommended fix.** Delete `CORE-049` from the `specify-adapters` fork; implement `adapter-extension-crate-missing` (extend the adapter indexer to record `extension`-declared + `extension/` dir + committed `adapter.wasm` presence facts, add the CORE rule, wire the check) and add the D10/D11 lint coverage. More broadly, the fork of *framework* rules (not just shared prose) into the adapters repo is a standing drift surface — consider generating/syncing the `rules/core` subset rather than hand-forking it, or scoping which CORE rules ship to the adapters profile.

### M4 — User-facing docs still teach the retired `tools[]` / `tools.yaml` model {#m4}

`specify/docs/explanation/tool-declarations.md` documents the **adapter-scope** declaration as a `tools:` array and a `tools.yaml` sidecar resolved by `load::plugin_sidecar()`:

```yaml
# docs/explanation/tool-declarations.md:46 (excerpt) — RETIRED shape
tools:
  - name: contract
    version: 0.3.0
```

D11 replaced this with the singular `extension:` object, and `additionalProperties: false` makes the documented shape **schema-invalid** — an author following this page produces a manifest the loader rejects. The page also references the retired `make use-local-dev` WASI-sidecar flow. (The *project-scope* `tools:` in `.specify/project.yaml` is still valid via `extension.schema.json`; only the adapter-scope section is stale.) Related stale references appear in `schemas/README.md` ("pins every `tools[].version`"), `specify-cli/docs/standards/workflow.md`, and `DECISIONS.md` sidecar prose. Note `make lint` passes because this explanation page is outside the linted deployable surface — so the drift is invisible to CI.

**Recommended fix.** Rewrite the adapter-scope section of `tool-declarations.md` to the singular `extension:` object + committed `adapter.wasm` + bundled-at-publish model; sweep `schemas/README.md`, `workflow.md`, and `DECISIONS.md` for residual `tools[]`/`tools.yaml`/sidecar language.

---

## Low

### L1 — `specify-cli/AGENTS.md` crate-graph predates the rename {#l1}
The top-of-file crate graph still lists `specify-tool-manifest` and `specify-tool` (lines 14–18) and the `specify-workflow` dependency line cites `tool-manifest`; the actual crates are `extension-manifest` and `registry`. The new transport modules (`crates/registry/src/{pack,oci,store}.rs`) are absent from "Modules of note." Since AGENTS.md is the agent's primary map (and RFC-49 will make it the unified repo's map), this should track the rename. The `adapter_uri.rs` bullet *was* updated, so the drift is partial.

### L2 — Vestigial local-dev tooling (RFC-49 T4 not yet started) {#l2}
`specify/scripts/use-local-dev.rs` still builds WASI tool sidecars from `cli_root.join("wasi-tools")` — a directory removed from `specify-cli`. The `Makefile` `use-local-dev` help still says "build WASI tools … write tools.yaml sidecars." RFC-49 T4 designates these for deletion; they are currently dead/misleading. Expected, since RFC-49 has not been executed, but worth tracking as cleanup debt.

### L3 — The store path still copies into the per-project manifest cache {#l3}
Even on the `specify:<name>@<ver>` path that *does* populate the store, `init` still runs `cache_adapter` → `refresh_cached_adapter`, copying the store entry into `<project-cache>/manifests/…`. D5 is explicit: a `Cached`/`Store` adapter "resolves directly to its store entry … with no per-project symlink or copy." The redundant copy is harmless but contradicts the decision and keeps the manifest-cache alive that D5 set out to retire (the plan lists this retirement as "REMAINING").

### L4 — Uncommitted `.gitignore` change drops several ignores {#l4}
`specify-cli/.gitignore` has an unstaged edit that removes `/specify`, `.specify/.cache/`, `.specify/project.yaml`, `stderr.log`, and `stdout.log` from the ignore set (re-ordering `target/` and `.DS_Store`). If committed as-is, local scaffolding/log files could be accidentally tracked. Confirm intent before committing.

### L5 — The forked `shared/` is already drifting at byte level {#l5}
Beyond [M3](#m3), `adapters/shared/rules/universal/README.md` already differs byte-for-byte between `specify` and `specify-adapters`. D12's "each repo owns its copy, free to diverge" makes this legal, but for *framework* rules (which must match the binary's expectations) silent divergence is a hazard. Consider a checked sync for the `rules/` subset.

---

## RFC-48 decision conformance matrix

| Decision | Status | Notes |
| --- | --- | --- |
| **D1** Packaging format / deterministic pack | Partial | Pack is byte-deterministic (`HeaderMode::Deterministic`, mtime/uid/gid=0, fixed modes, `ZSTD_LEVEL=19`) and symlink-dereferencing — correct and unit-tested. Shipped as a **single** packed OCI layer, not the RFC's two-layer (prose + wasm) working default (see [O1](#opportunities)). |
| **D2** Immutable fetch locator | Done | `AdapterPackageRef` parses `specify:<name>@<semver>`; exact-semver, no branch defaulting; `adapter-package-ref-version-required`. |
| **D3** Self-contained artifact | Blocked | Symlink-deref logic is correct, but [C1](#c1) means the artifact is never actually produced for the two adapters that have shared content. `vendor_spec_runtime` correctly retired. |
| **D4** Digest verify-on-read | **Not done** | Publish-only round-trip verify; no install-metadata, no read-time verify; `store::install` dead. See [H2](#h2). |
| **D5** Global store, resolved in place | Partial | Store root/entry resolver, flock+temp+chmod+atomic-rename install, and `AdapterLocation::Store` probe are implemented and tested. But shorthand init never populates it ([H3](#h3)), the manifest-cache copy persists ([L3](#l3)), and the impl added a separate `Store` variant rather than reusing `Cached` as the RFC described. |
| **D6** Publish tooling | Partial | `specify adapter publish` does pack→push→pull-back→verify with an `adapter-republish-conflict` guard — sound. The release job exists but cannot succeed ([C1](#c1)), and basic-auth is misconfigured ([M2](#m2)). |
| **D7** Adapter repo extraction | Partial | `contracts`/`vectis` + co-located `extension/` crates + committed `adapter.wasm` are in `specify-adapters`, and `specify`'s `targets/` is drawn down to `omnia` (+ sources). But the move is functionally broken ([C1](#c1)) and the shared fork is incomplete. |
| **D8** Registry visibility / pull auth | Partial | Token/basic/anonymous env auth exists; "authenticated by default" is not enforced (anonymous fallback), and the basic-auth var name is wrong ([M2](#m2)). |
| **D9** `build` packs self-contained artifact | Partial | `specify adapter build` (`--dry-run`, `--refresh-extension`) + exclude set + symlink-deref present and correct in isolation; blocked by [C1](#c1) on the real trees; no integration tests for the verb. |
| **D10** Co-located extension source | Done* | Sparse workspace (`members = [".../extension", …]`), committed wasm, conditional `wasm32-wasip2` compile. *Missing the `adapter-extension-crate-missing` guard ([M3](#m3)) and a compile→committed-wasm integration test. |
| **D11** Extension declaration in manifest | Done | Singular `extension` object in all three schemas (`additionalProperties:false`, rejects `version`/`source`/`sha256`), `Option<AdapterExtensionDeclaration>`, unified `{read,write}` `ExtensionPermissions`, `tools.yaml` reader retired, `extension run` resolves from the adapter tree. Solid. Docs lag ([M4](#m4)). |
| **D12** In-repo shared content | **Not done** | The fork into `specify-adapters/shared/` was not performed; symlinks dangle ([C1](#c1)). |

\* "Done" = code-complete and behaviourally correct in isolation; several are still gated by C1/H1.

---

## Opportunities & optimizations {#opportunities}

- **O1 — Two-layer OCI (D1 working default).** The implementation packs prose+wasm into one layer. The RFC's stated benefit of the two-layer shape is that "an extension-only rebuild re-pushes just the wasm layer." With a single layer, every prose typo re-pushes the full tree including the 6.5 MB vectis wasm, and vice versa. Single-layer is defensible (simpler, and the spike showed `wkg` rejects opaque blobs), but the deviation from the RFC's working default should be recorded in `DECISIONS.md`, and two-layer revisited if churn proves costly.
- **O2 — Per-call tokio runtime.** `oci.rs` builds a fresh current-thread runtime per push/pull (`build_runtime` + `block_on`). Fine for one-shot CLI calls; if a future verb publishes many adapters in one process (the release loop shells out per adapter today), a shared runtime would help.
- **O3 — Blocking install lock vs `try_lock`.** `store::install_layer` uses blocking `File::lock()`, while the RFC/plan referenced the `File::try_lock` family from `plan_lock.rs`. Blocking is arguably better for install (wait rather than fail), but the divergence from the cited precedent is worth a one-line rationale.
- **O4 — `Store` vs `Cached` duplication.** Two probe rungs and two near-identical resolution arms exist while the manifest cache is being retired. Once [L3](#l3)/D5 retirement completes, collapsing to a single store-backed arm would simplify `locate_axis` and the `AdapterLocation` enum.
- **O5 — Committed binary weight.** `vectis/adapter.wasm` is ~6.5 MB committed to git (contracts ~1 MB). The RFC accepts committed wasm (git content-addresses it, packing stays toolchain-free), but as adapters multiply, a periodic-refresh + `git` weight policy (or building wasm in release CI from the crate rather than committing) is worth a future decision.

---

## What's working well

To keep the picture balanced — the foundations are genuinely good:

- **Pack determinism + symlink dereference** is correct, normalized, and unit-tested (`pack_dereferences_symlinks_into_bytes`, the `extension/` exclusion test, the determinism path).
- **D11 reshape** is clean and complete end-to-end: schemas, Rust types, permission unification, sidecar retirement, and `specify extension run` resolving from the installed tree — with serde-level rejection tests.
- **The publish state machine** (pack → republish-guard → push → pull-back → verify) is well-reasoned, including idempotent same-digest republish and a hard conflict on different bytes.
- **Store install mechanics** (temp-under-root, flock, recursive read-only, atomic rename, idempotent concurrent installs) match the staged-install precedent and are tested.
- **The prose repo is clean**: `make lint` reports 0 findings, so `omnia` + the source adapters are schema-valid and link-clean under the new `extension` schema, and `vendor_spec_runtime` was retired without collateral.

---

## Suggested remediation order

1. **C1** — perform the D12 shared-content fork (clean-copy prose into `specify-adapters`); add a `specify adapter build --dry-run` smoke gate so it cannot regress. *(Unblocks D3/D6/D7/D9 and the release job.)*
2. **M1** — fix the contradictory `adapter`-verb test. *(Unblocks `cargo make ci`.)*
3. **H1** + **M3** — make `lint framework` adapters-aware, delete stale `CORE-049` from the fork, implement `adapter-extension-crate-missing`. *(Unblocks the T5 CI seam and gives the adapters repo real coverage — which would itself catch C1.)*
4. **M2** — converge the registry auth env var name across code, CI, and README.
5. **H2** — finish (or formally defer + dead-code-delete) D4 verify-on-read.
6. **H3 / M4 / L1–L5 / O1–O5** — close the documentation/default-path gaps and record the deliberate deviations in `DECISIONS.md`.

---

### Appendix — reproduction

```bash
# Prose repo is clean
cd specify && make lint                      # => 0 findings

# Build the rfc-48 binary once (used below)
SPEC=../specify-cli/target/release/specify

# C1: extracted adapters cannot pack
cd ../specify-adapters
$SPEC adapter build --path adapters/targets/contracts --dry-run   # => adapter-pack-failed
$SPEC adapter build --path adapters/targets/vectis    --dry-run   # => adapter-pack-failed

# H1: adapters repo is not a framework root
$SPEC lint framework --framework-root .                            # => not a framework root: .

# C1 root cause: dangling shared-content symlinks
find adapters/shared/references/runtime -type l | while read l; do [ -e "$l" ] || echo "DANGLING: $l"; done
```
