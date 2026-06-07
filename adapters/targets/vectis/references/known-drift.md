# Known drift backlog

This catalogue has two readers:

- **Operators** triaging a `deferred` outcome from [`../briefs/build.md`](../briefs/build.md) § Template / version-pin drift handling, or a Mode 3 cap-matrix failure from [`../briefs/merge.md`](../briefs/merge.md). Match the symptom to one of the items below, then escalate via a template-fix slice rooted in the CLI repo.
- **The host-side template-updater workflow** living in `augentic/specify-cli`'s vectis carve-out. The workflow checks this list **first** when a fresh `specify vectis update-versions --verify` run fails: if the reproduced failure matches an item below, the workflow follows that item's playbook rather than re-diagnosing from scratch.

Concrete fix items in this catalogue from Vectis template-update work. Each item is reproducible today via:

```sh
specify vectis update-versions --dry-run --verify
```

Run via `<specify-cli>/target/debug/specify` after `cargo build -p specify` in the `specify-cli` checkout, or via an installed `specify` binary, against live registries.

When a fix ships, remove the item from this file in the same commit that lands the fix. Stale entries make the catalogue softer than intended.

---

## 1. `uniffi` / `cargo-swift` decoupling from `crux_core::cli::bindgen`

**Symptom**: `verify::core::run_pipeline`'s `codegen kotlin` step fails with `unresolved module path shared::ffi` on a fresh scaffold built against `uniffi = "=0.31.0"` + `cargo_swift = "0.11"`.

**Root cause**: `crux_core 0.17.0`'s `cli` feature transitively pins `uniffi_bindgen = "=0.29.4"`. `<specify-cli>/templates/vectis/core/codegen.rs` calls `crux_core::cli::bindgen(...)` for Kotlin bindings. Mixing a 0.31 runtime (in `shared`) with a 0.29 bindgen fails. There is no Crux release today that tracks `uniffi_bindgen 0.31`.

**Why it cannot be hotfixed by pin bumping**: tried and reverted. The Xcode 26+ iOS concern that originally motivated the bump (`import sharedFFI` failing under cargo-swift 0.9's bindgen output) is already addressed by bumping the *per-developer* cargo-swift install to 0.11; cargo-swift 0.11's bindgen reads uniffi 0.29.4 metadata correctly because the metadata format is stable across 0.29 → 0.31.

**Fix path** (structural, scoped by this item):

1. Rewrite `<specify-cli>/templates/vectis/core/codegen.rs` to call the `uniffi_bindgen` crate directly rather than `crux_core::cli::bindgen`. This requires:
   - Adding `uniffi_bindgen = "=0.31.0"` to the shared crate's `[dependencies]` behind the `codegen` feature (same feature gate `crux_core::cli::bindgen` sits behind today).
   - Reconstructing the argument set currently passed via `BindgenArgsBuilder` against uniffi_bindgen's public API for `run_pipeline(...)`. Read `https://github.com/mozilla/uniffi-rs/blob/v0.31.0/uniffi_bindgen/src/lib.rs` for the 0.31 surface.
   - Keeping the Swift path untouched (`crux_core::typegen` for Swift types is orthogonal to the uniffi bindgen call).
2. Bump `<specify-cli>/crates/vectis/embedded/versions.toml`:
   - `uniffi = "=0.31.0"`
   - `cargo_swift = "0.11"`
3. Update the rationale comment block in `embedded/versions.toml` to note the decoupling -- the existing block says "mixing 0.31 runtime with 0.29 bindgen fails", which is no longer true once step 1 lands.
4. Validate against the full cap matrix via `specify vectis update-versions --dry-run --verify`.

**Alternative**: wait for a `crux_core` release that tracks `uniffi_bindgen = "=0.31.0"`. In that case the fix is a plain pin bump in `<specify-cli>/crates/vectis/embedded/versions.toml`, no template surgery.

---

## 2. AGP 9.x vs `rust-android-gradle 0.9.6`

**Symptom**: `specify vectis update-versions --dry-run` auto-proposes `android.agp = "9.1.1"` (latest on Google Maven). Scaffolding with that pin fails during `gradle wrapper` bootstrap (or later, during `:app:assembleDebug`) with a trace containing `java.lang.NoSuchMethodError: org.gradle.api.internal.file.copy.CopySpecInternal.setFileMode(Integer)`.

**Root cause**: AGP 9.x requires Gradle 9.x, which removed the `setFileMode(Integer)` method. `rust-android-gradle = 0.9.6` still calls it during plugin classpath evaluation. Any flow that loads the `rust-android-gradle` plugin on Gradle 9.x throws during project evaluation, which is before the wrapper task can even run.

**Fix path** (one of two):

- **Cap AGP below 9.0 in `update-versions`**: teach `update_versions::query::google_maven_latest_stable` (or the coordinate-specific query fn for AGP) in `<specify-cli>/crates/vectis/src/update_versions/query.rs` to filter on a max-version constraint `< 9.0`. Add the cap as a constant in `update_versions::query` with a comment citing this entry. This is the narrower fix and correctly reflects that *Vectis's Android plugin choice* is what blocks AGP 9.x, not a user preference.
- **Drive `rust-android-gradle` upstream**: file or contribute a PR that removes the `setFileMode(Integer)` call. When a new release ships, drop the cap added above.

Gradle-the-wrapper is pinned to 8.13 via a scratch bootstrap (`init::android::bootstrap_wrapper` in `<specify-cli>/crates/vectis/src/init/android.rs`), so the bundled wrapper works today. The cap prevents the *system* Gradle on the developer's machine from being driven into 9.x territory by a stale AGP pin. Do not remove the bootstrap workaround when this entry is fixed -- the two are independent.

---

## 3. RUSTSEC advisories pulled in by the `sse` cap

**Symptom**: the `http,kv,time,platform,sse` matrix combo fails `cargo deny check` with:

- `RUSTSEC-2024-0384` -- `instant` unmaintained.
- `RUSTSEC-2026-0097` -- `rand 0.7.3` unsound.

**Root cause**: `async-sse 5.1.0` transitively pulls `http-types 2.12.0` → `rand 0.7.3` and `futures-lite 1.13.0` → `instant 0.1.13`. Neither `async-sse` nor `http-types` has a maintained successor that drops these.

**Fix path**:

1. Extend `<specify-cli>/templates/vectis/core/deny.toml`'s `[advisories] ignore` array with the two entries below (preserve any existing entries):

   ```toml
   [advisories]
   ignore = [
       # transitive via async-sse -> http-types -> rand 0.7.3 when the sse
       # cap is enabled; no maintained upstream replacement today.
       "RUSTSEC-2026-0097",
       # transitive via async-sse -> futures-lite 1.13 -> instant; no
       # maintained upstream replacement today.
       "RUSTSEC-2024-0384",
   ]
   ```

   Preserve the existing `RUSTSEC-2024-0370` (`proc-macro-error`) and `RUSTSEC-2025-0141` (`bincode 1.x`) entries with their rationale comments; they cover a different transitive chain.
2. Validate the full cap matrix -- the `sse`-including combos must now pass `cargo deny check`, and no previously-passing combo should regress.
3. Leave a note in this file that each ignore is only valid as long as the `async-sse` chain remains the only source. If a future Crux release drops `async-sse` (replacing it with a maintained SSE library) these two entries must come out of the list in the same commit.

**Alternative**: push Crux upstream to drop the `async-sse` transitive chain. When that lands, remove both advisories from `deny.toml` in the same commit as the Crux pin bump.

---

## 4. `facet_generate` req-string normalisation (cosmetic)

**Symptom**: `specify vectis update-versions --dry-run` proposes `crux.facet_generate = "^0.15"` (copying the string verbatim from `crux_core 0.17.0`'s published `Cargo.toml` dep req). The current pin is `=0.15`. Cargo treats them as equivalent; the diff is purely textual.

**Fix path** (choose one):

- **Normalise on query**: in `update_versions::query::crates_io_latest_stable` (or a wrapper) inside `<specify-cli>/crates/vectis/src/update_versions/query.rs`, convert `^0.x` / `^0.x.y` to `=0.x.y` before emitting. This preserves the Vectis hard-pin convention for every Crux-adjacent coordinate.
- **Document the exception**: if `facet_generate` is legitimately minor-pinned upstream, change `<specify-cli>/crates/vectis/embedded/versions.toml` to carry `= "^0.15"` and update the hard-pin rationale block to note that `facet_generate` tracks semver-minor. The hard-pin set was vetted for `facet` + `crux_core` co-dependencies; `facet_generate` may follow a different policy.

This item is purely cosmetic -- no build or verify step actually fails. It is listed so `specify vectis update-versions` runs don't emit a noisy "changed" entry on every invocation when nothing has substantively moved.

---

## Sizing reminder

All four items above were concretely produced by a `specify vectis update-versions --verify` verification run. When this skill runs for the first time against live registries, the only item it is blocked from fixing mechanically is **item 1** -- that needs a code edit in `<specify-cli>/templates/vectis/core/codegen.rs` that is a meaningful piece of work. Items 2, 3, and 4 are short, scoped, and should land first.
