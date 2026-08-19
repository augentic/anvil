# ADR-0011: The launcher folds into the root deployment unit

> Status: **Accepted** (2026-08-19)
> Date: 2026-08-19

## Context

`crates/launcher` carries the native deployment policy of the shipped binary: project-root
anchoring, the mount directories, the pre-bound HTTP listener, the `/mcp/<axis>/<name>`
route hook, and the fail-closed adapters-only `GuestResolver`. It has exactly three
consumers, all inside the root package's orbit — `src/main.rs` (the shipped
`omnia::runtime!` invocation), `examples/runtime.rs` (the journey host, ADR-0009 §5), and
its own integration tests. It is `publish = false`, exports no product vocabulary, and can
never be consumed apart from the deployment unit it wires.

A workspace crate implies a product boundary. This one has none: the crate exists only
because the deployment unit's targets needed to share expressions. Invariant 4 (delete
before add) says the cheaper structure wins — the sharing the crate provides is available
from the root package's own library target.

## Options

1. **Keep the crate.** Status quo: one extra package, three extra DAG edges, and a
   crates map entry, all carrying no boundary.
2. **Publish the crate** so external native hosts consume the desktop policy. No such
   host exists; publishing desktop-specific policy (CWD anchoring, `~/.emery` layout)
   would freeze accidents into an API.
3. **Fold it into the root package** as a native-only library module. The root lib
   target (today a wasm32-only cdylib) gains `rlib` and a
   `#[cfg(not(target_arch = "wasm32"))] pub mod launcher`; binary, journey-host example,
   and tests consume `emery::launcher::…`.

## Decision

Option 3. The launcher is deployment wiring of the root Omnia unit, not a product crate:

- The sources fold into one native-only file (`src/launcher.rs`), public from the
  root lib behind `cfg(not(target_arch = "wasm32"))`. The wasm32 guest cdylib is
  unchanged; `cargo check --lib -p emery --target wasm32-wasip2` stays the guest
  compile gate.
- The fold applies two simplifications rather than moving verbatim. The `Policy`
  wrapper — a single-instance newtype over `ExecutionPaths` whose methods only
  delegated — collapses into the pure `launcher::assemble` seam returning
  `ExecutionPaths` directly. And the operator-set `HTTP_ADDR` listener override is
  deleted: nothing sets it, so `launcher::http_listener` always binds an ephemeral
  loopback port (the runtime still injects the guest-visible `HTTP_ADDR` from the
  bound address, so the adapter seam is unaffected).
- The embedded first-party registry generation (`EMERY_EMBED_DIR` → `embedded.rs`,
  ADR-0002 §2) merges into the root `build.rs` beside the engine embed.
- ADR-0009 §5 is preserved structurally: the shipped binary and the journey host link
  the *same* module from the *same* lib target, so the mounts and resolver wiring cannot
  drift apart.
- Mechanical gates move with the code: `tests/layering.rs` drops the three
  `emery-launcher` edges and gains the root's direct `emery-engine` / `emery-error`
  edges; the `scripts/ratchet.toml` `src` ceiling rises by the moved lines (this ADR is
  the required citation) and the `crates/launcher` entry is deleted.

## Deletions

The `emery-launcher` package (manifest, workspace-dependency entry, crates map row) and
its three layering edges; the `Policy` type; the operator-set `HTTP_ADDR` bind override
(the guest-visible injected `HTTP_ADDR` is unchanged). Concept-count effect: −1 workspace
crate, −1 exported type, −1 environment knob; no verb, noun, artifact, or envelope field
moves.

## Consequences

- The root `src/` ratchet ceiling rises from 46 to the moved total (~340 non-blank
  lines); the workspace total shrinks slightly (one manifest and one build script fewer).
- `EMERY_EMBED_DIR` changes now re-run the root build script, which re-invokes the
  (cargo-cached) child wasm32 engine build before regenerating the registry table —
  a small journey-rung cost accepted for one build script instead of two.
- The root lib is now `["cdylib", "rlib"]`; the rlib exists solely so the package's own
  binary, example, and test targets link the shared module.

## Revisit trigger

A second native host outside this repository needs the desktop deployment policy as a
library — re-extract it as a published crate through a new ADR, with the API designed
for that consumer rather than inherited from these expressions.
