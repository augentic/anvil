# Lint override inventory and removal plan

Working notes from a full-repo scan of `#[expect]` / `#![allow]` (and workspace-level clippy `allow`) in `augentic/emery` and `augentic/emery-adapters`. Address each item independently; suggested order is at the bottom.

House posture (`docs/standards/coding-standards.md`): site-local suppressions are `#[expect(<lint>, reason = "…")]` at the smallest scope; module-root suppressions stay `#![allow(<lint>, reason = "…")]`. Refactor first; a suppression is the leftover when a refactor is infeasible.

Scan date: 2026-08-25. Six site-level suppressions (three emery, three emery-adapters) plus one workspace-level clippy allow in each repo. No other `#[allow]` / `#[expect]` in `.rs` sources.

---

## Inventory

### emery — site overrides

| ID | File | Form | Lint(s) | Recommendation |
| --- | --- | --- | --- | --- |
| E4 | `crates/prose/src/lib.rs` | `#[expect]` | `clippy::disallowed_methods` | After config split (S1); otherwise relocates |
| E5 | `crates/engine/src/cli.rs` | `#[expect]` | `clippy::disallowed_methods` | Keep unless product change |
| E6 | `crates/adapter/src/source.rs` | `#![allow]` (inner `generated` module) | `missing_docs`, `unsafe_code`, `clippy::pedantic`, `clippy::nursery` | Keep — generated-code fence |

### emery-adapters — site overrides

| ID | File | Form | Lint(s) | Recommendation |
| --- | --- | --- | --- | --- |
| A1 | `examples/eval/src/main.rs` `Paths::locate` | `#[expect]` | `clippy::disallowed_methods` | After config split (S2) |
| A2 | `examples/eval/src/main.rs` `Paths::component` | `#[expect]` | `clippy::disallowed_methods` | After config split (S2) |
| A3 | `examples/eval/src/main.rs` `run_case` | `#[expect]` | `clippy::disallowed_methods` | After config split (S2) |

### Workspace-level allow (not a site attribute)

| ID | File | Lint | Recommendation |
| --- | --- | --- | --- |
| W1 | emery `Cargo.toml` `[workspace.lints.clippy]` | `multiple_crate_versions = "allow"` | Keep until a duplicates audit shrinks the tree |
| W2 | emery-adapters `Cargo.toml` `[workspace.lints.clippy]` | `multiple_crate_versions = "allow"` | Same as W1 |

Related cargo-deny policy (not rustc/clippy attributes): both repos' `deny.toml` set `multiple-versions = "allow"` and `wildcards = "allow"`. Touch only if you are also revisiting W1/W2.

### Config splits (unlock several site overrides)

| ID | Change | Unlocks |
| --- | --- | --- |
| S1 | Narrow emery `crates/clippy.toml` guest deny-list so host crates under `crates/` do not inherit it | E4 (and possibly cleaner prose/testkit story) |
| S2 | Move emery-adapters guest deny-list off the repo-root `clippy.toml` onto `sources/` | A1, A2, A3 |

Four of the six site overrides exist because the guest deny-list is scoped by directory, not by target.

---

## E4 — `emery_prose::emit` env reads

**File:** `crates/prose/src/lib.rs`

```rust
#[expect(
    clippy::disallowed_methods,
    reason = "build-script crate; cargo communicates CARGO_MANIFEST_DIR and OUT_DIR through the env"
)]
pub fn emit(tree: &str) {
```

**Why it fires:** `emit()` calls `std::env::var` for `CARGO_MANIFEST_DIR` and `OUT_DIR`. `crates/clippy.toml` bans those methods for every crate under `crates/`, including this host build helper. `emit_from(root, out_dir)` already takes paths and does not need the expect.

**Call sites today:**

- emery `crates/engine/build.rs` — `emery_prose::emit("prose")`
- emery-adapters `sources/{documentation,typescript,intent}/build.rs` — same

**Remove by:**

- **Preferred with S1:** once prose no longer inherits the guest deny-list, delete the `#[expect]` and keep `emit()`.
- **Without S1:** delete `emit()`, have each `build.rs` pass dirs into `emit_from`. That **moves** the env read into those build scripts. Adapter `build.rs` files still inherit emery-adapters' deny of `env::var` (S2), so the suppression relocates rather than disappearing.

**Done when:** no `disallowed_methods` expect remains on `emit`, and adapter/engine build scripts do not grow a matching expect unless S2 is also done.

---

## E5 — guest CLI `NO_COLOR` / `TERM`

**File:** `crates/engine/src/cli.rs` (`error_style`)

```rust
#[expect(
    clippy::disallowed_methods,
    reason = "the guest is the CLI (wasi:cli/run); NO_COLOR/TERM are the terminal \
              colour convention, not app configuration"
)]
fn error_style() -> (&'static str, &'static str) {
    let opted_out = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty())
        || !std::env::var_os("TERM").is_some_and(|term| !term.is_empty() && term != "dumb");
```

**Why it fires:** this is guest code. The deny-list is doing its job. The expect is a policy carve-out: env for the terminal-colour convention is OK; env for app configuration is not. On wasm, `stderr_terminal()` is hardcoded `true`, so colour is gated only by those env guards.

**Remove by (product change, not a docs fix):** stop reading env in the guest — WASI terminal probe, a host-passed colour flag, or drop ANSI. S1 does **not** unlock this; engine stays a guest crate.

**Recommendation:** keep until there is a real terminal/host signal.

---

## E6 — wit-bindgen generated module

**File:** `crates/adapter/src/source.rs`

```rust
mod generated {
    #![allow(
        missing_docs,
        unsafe_code,
        clippy::pedantic,
        clippy::nursery,
        reason = "wit-bindgen generated bindings are not hand-maintained; the generated code cannot carry this workspace's lint posture"
    )]
    wit_bindgen::generate!({ ... });
}
```

**Why it exists:** generated bindings cannot meet the workspace lint bar. This is the intended generated-code fence (house example in coding-standards still mentions a deleted `generated.rs` / `agents` command; this module is the live analog).

**Remove by:** do not lint-clean generated bindings. Relocating into a tiny crate with crate-level allows is still an override, just not next to hand-written code.

**Recommendation:** keep.

---

## A1 / A2 / A3 — eval runner host env and clock

**File:** `examples/eval/src/main.rs` (emery-adapters)

All three are `clippy::disallowed_methods`. Repo-root `clippy.toml` applies the guest deny-list (`env::var` / `var_os`, `Instant::now`) to `examples/eval` as well as `sources/*`.

| ID | Site | What it covers |
| --- | --- | --- |
| A1 | `Paths::locate` | `EMERY_REPO`, `CARGO_TARGET_DIR`, `EMERY_BIN` via `var_os` |
| A2 | `Paths::component` | `CARGO_TARGET_DIR` via `var_os` |
| A3 | `run_case` | `Instant::now()` for scorecard wall-clock |

Eval is a public-contract **host** binary (spawns the shipped `emery` CLI). It should not inherit the guest deny-list.

**Remove by:** do S2, then delete all three `#[expect]`s. Do not thread env into `main` while the deny-list still covers the eval crate — the calls still fire.

`std::env::args()` in eval is already fine: adapters' deny-list does not include `args`.

**Done when:** the three expects are gone and `cargo make lint` in emery-adapters is clean.

---

## S1 — emery: guest deny-list scope

**File:** `crates/clippy.toml`

**Problem:** Clippy walks from the package dir upward. Every crate under `crates/` inherits this deny-list (`std::env::var`, `Command`, `Instant::now`, `OnceLock`, …), including host-only `emery-prose` and native `emery-testkit`. Engine and adapter are compiled both as wasm guests and as native test code, so a crate-level file cannot be target-gated.

**Options:**

1. Move the deny-list to only the packages that are actually guests, accepting that their native test builds still see it (status quo for engine/adapter; helps prose).
2. Keep `crates/clippy.toml` for guests and add per-crate `clippy.toml` overrides for prose (empty or host-oriented). Clippy uses the closest file plus parents — verify merge behaviour before relying on a child file to *undo* parent `disallowed-methods`.
3. Leave S1; keep E4 as a documented host carve-out.

**Unlocks:** E4. Does not unlock E5 (engine is a guest) or E6.

**Done when:** `emery-prose` can call `std::env::var` in `emit()` without an expect, and guest crates still fail clippy on ambient env/process/time.

---

## S2 — emery-adapters: guest deny-list off the repo root

**File:** emery-adapters `clippy.toml` (repo root)

**Problem:** the same guest deny-list applies to `sources/*` (wasm adapters) and `examples/eval` (host runner).

**Remove by:** move `disallowed-methods` / `disallowed-types` from the root file into `sources/clippy.toml` (or per-adapter). Leave root `clippy.toml` as `doc-valid-idents` (and any other host-safe settings).

**Unlocks:** A1, A2, A3.

**Done when:** eval compiles without `disallowed_methods` expects; a deliberate `std::env::var` in an adapter source crate still fails clippy.

---

## W1 / W2 — `multiple_crate_versions`

**Files:**

- emery `Cargo.toml` `[workspace.lints.clippy]` (`multiple_crate_versions = "allow"`)
- emery-adapters `Cargo.toml` (same)

**Why it exists:** transitive duplicates accrete from upstream (windows-sys generations, wasmparser/wit-\* cycles, rand v0.8/v0.9). A clippy.toml ratchet allowlist grew without shrinking. The intended substitute is a periodic `cargo tree --duplicates` audit.

**Remove by:** unify/pin transitives until clippy is quiet, or accept permanent noise. Not a site-level refactor.

**Recommendation:** keep until an audit actually shrinks the tree. If you touch this, look at matching `deny.toml` `multiple-versions = "allow"` in the same change.

---

## Suggested order

Independent, cheapest first:

1. **S2** then **A1–A3** — adapters deny-list onto `sources/`, delete eval expects. Lives in emery-adapters.
2. **S1** then **E4** — narrow `crates/clippy.toml`, drop `emit()` expect (or delete `emit()` only if S1 is rejected).
3. Leave **E5**, **E6**, **W1**, **W2** unless you are changing colour policy, generated bindings, or duplicate-crate policy.

Verify each with `cargo make lint` in the repo you touched (`cargo make ci` before commit).
