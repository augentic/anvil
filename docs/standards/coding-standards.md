# Coding standards

The external baseline is the [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/guidelines/index.html) (and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) they build on): follow it for anything this document and [style.md](./style.md) do not address. Every section below is a house delta — a project contract, a sharper rule, or an explicit override — and where a section disagrees with the baseline, this document wins. Enforced by clippy (`make lint`) and review. When a rule fights you, add the case to the rule with a before/after — don't carve out a local exception.

## Lints

Workspace lints live in `Cargo.toml`. Defaults are aggressive — clippy `all`/`cargo`/`nursery`/`pedantic` are all `warn`, plus a curated set of `restriction` lints and a tightened rust lint set (`missing_debug_implementations`, `single_use_lifetimes`, `redundant_lifetimes`). Compile under `RUSTFLAGS=-Dwarnings` (`make test` does this), so any new warning fails CI.

Visibility on internal items follows clippy's `redundant_pub_crate` (nursery) rather than rustc's `unreachable_pub`: prefer bare `pub` and let the parent module's privacy do the constraining. The two lints are mutually exclusive — enabling both would loop. `unreachable_pub` stays at its allow-by-default, and any `#[expect(unreachable_pub, …)]` carve-out is a rot signal, not a tool you reach for.

Doc idents such as `GitHub`, `MiB`, `OAuth`, `OpenTelemetry`, `SemVer`, `WebAssembly`, and `YAML` live in `clippy.toml` `doc-valid-idents`. Suppression rules are in [Lint suppression posture](#lint-suppression-posture) below.

`taplo.toml` formats `Cargo.toml` files. Dependency arrays under `*-dependencies` and `dependencies` reorder alphabetically; preserve that on edit.

## Lint suppression posture

Site-local suppressions are `#[expect(<lint>, reason = "…")]` at the **smallest possible scope**, not `#[allow]` — a dead `#[expect]` is a build failure, so the suppression cannot rot (the baseline's M-LINT-OVERRIDE-EXPECT). The house additions: module-level suppressions stay `#![allow(<lint>, reason = "…")]` because lint-rot detection at the module root is not useful (the suppression typically covers many sites), and identical `reason = "…"` strings across three or more files mean you should promote a single `#![allow]` to the parent module — the file-level repetition is noise, not signal.

```rust
// BAD — site-local #[allow]
#[allow(clippy::cognitive_complexity, reason = "linear state machine")]
fn step(...) { ... }

// GOOD — same scope, #[expect]
#[expect(clippy::cognitive_complexity, reason = "linear state machine")]
fn step(...) { ... }

// GOOD — module-root suppression that legitimately covers every item below
// crates/engine/src/generated.rs
#![allow(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    reason = "binary-internal context-fence code consumed only by the `agents` command; documenting ~30 internal fields adds noise, not API surface"
)]
```

## Comments

Comments answer "why does this look like this *today?*" — non-obvious intent, trade-offs, or constraints the code itself can't convey. Migration trails, old labels, and "this used to be X" rationale belong in commit messages — not in code or doc comments. Doc comments on items that surface in `--help` (clap `#[derive]` fields) must be operator-facing one-liners; rationale moves below the derive block where it doesn't leak into help output.

Density caps are **review only** — clippy and rustfmt cannot express them. They apply to Rust sources and to WIT contracts (`wit/`, `crates/*/wit/`):

- **Module `//!` docs** answer "what is this module today?" in **1–3 prose lines**. No deployment tours, no AGENTS.md restatements, no RFC archaeology — the crate graph and the workflow contract already own that prose; a module doc that repeats it goes stale and buries the one line the reader needed.
- **Item `///` docs** keep the overview under **~8 lines** before any `#` section. `# Errors` / `# Panics` sections may list discriminants; keep each bullet one line.
- **`//` comments** run **≤ 3 consecutive lines**. A tip lives next to the surprising branch it explains, never inside a preamble essay.
- **Historical phrases** are banned in comments and docs: `Phase `, `formerly`, `previously lived`, `old contract`, `former tests`, `to avoid the`. Git history is the record.

```rust
// BAD
//! Per the workspace split 2.9 ("Specify wires components, not adapters"),
//! `specify` commits only the generation documents — `spec.md` plus
//! `design.md`. The pre-Phase-3.7 filename was `charter.md`;
//! Historical rename detail belongs in git history, not module docs.

// GOOD
//! Commits `spec.md` and `design.md` as one generation. Other state
//! is minted by its owning verbs, not by `specify`.
```

The composition-root failure mode is the essay that restates architecture and hides the tip. Collapse the essay; keep the tip at the site that needs it:

```rust
// BAD — 22-line //! deployment tour restating AGENTS.md, with the one
// operational fact (the read-only project mount) buried in the middle.

// GOOD
//! The shipped `emery` executable: one `omnia::runtime!` invocation.

// …inside the macro body:
// The invocation directory mounts read-only — nothing writes the tree.
mounts: [{ name: ".", path: "." }],
```

Doc comments describe what this is today. Version-history tables, dated bumps, commit hashes, and migration notes belong in git log — not in `///` blocks. Longer prose belongs in the standards docs.

`cargo doc` is part of `make ci`, so doc comments must compile. Reference paths inside backticks (`` `Self::config_path` ``) are fine; bare links (`[Foo]`) need a corresponding intra-doc target or rustdoc fails the build.

## Naming

Prefer short, idiomatic Rust names. Don't restate context the surrounding module, type, or function already supplies. Avoid `_local` / `_value` / `_helper` suffixes. New functions: 1–3 words. Predicates start with `is_` / `has_`. DTOs returned by handlers are `<Action>Body` / `<Action>Row`, never `<Action>Response` / `<Action>Json` (the type's role is `Body`; the format dispatch lives in `emit` — see [handler-shape.md](./handler-shape.md)).

**Identifier length.** Declared item names (`fn` / `struct` / `enum` / `trait` / `type` / `const` / `static` / `mod`), named fields, and enum variants are **≤ 25 characters** (Unicode scalars on the bare identifier, not the module path). **Review only** — clippy has no identifier-length lint (`module_name_repetitions` still catches in-module restatement). Push narrative into docs, comments, or nested `mod` context — not into the identifier.

A function defined in `mod <name>` (or `commands/<name>.rs`) MUST NOT carry `<name>` as a suffix or prefix on its own name — the module path already supplies that context. Clippy's `module_name_repetitions` (on by default through the `pedantic` group) catches this at lint time.

```rust
// BAD — file is commands/registry.rs / mod registry
fn show_registry(ctx: &Ctx) -> ... { ... }
fn validate_registry(ctx: &Ctx) -> ... { ... }
fn add_to_registry(ctx: &Ctx) -> ... { ... }

// GOOD — caller writes registry::show, registry::validate, registry::add
fn show(ctx: &Ctx) -> ... { ... }
fn validate(ctx: &Ctx) -> ... { ... }
fn add(ctx: &Ctx) -> ... { ... }
```

## Brevity

The codebase optimises for short reading over short writing. Concretely:

- **Names**: 1–3 words. Predicates start with `is_` / `has_`. Avoid `_local` / `_value` / `_helper` / `_path` / `_dir` suffixes when the parameter type or surrounding context already says so (`is_slot(p: &Path)`, not `is_slot_path`).
- **Cross-module redundancy**: `WorkspaceBranchPreparationFailed` inside `Error` reads as `Error::WorkspaceBranchPreparationFailed` — drop the `Workspace` prefix when every variant in the cluster already operates on a workspace. Clippy's `module_name_repetitions` catches the in-module cases; cross-module redundancy is on you and reviewers.
- **One-variant enums** are dead overhead. Drop the variant or the enum. If the type's name already discriminates, the enum adds nothing.
- **Field prefixes**: a struct named `RegistryAmendmentArgs` does not carry `proposed_` on every field — the struct name already says "proposal".
- **Comment redundancy**: don't paraphrase a `match` arm's variant in a `// …` comment when the variant's doc-comment already explains it.

Reviewers catch the density caps (see [Comments](#comments)) and the 25-character identifier cap (see [Naming](#naming)). Clippy's `module_name_repetitions` catches the in-module restatement cases.

## Format dispatch

Operations do **not** open-code `match format { Json, Text }`. They return typed bodies; the command projector in `crates/engine/src/cli.rs` owns format dispatch through the internal `emit` function in the same module. Operations never pick a sink directly. See [handler-shape.md](./handler-shape.md) for the operation and projector contract.

```rust
// BAD
match format {
    Format::Json => serde_json::to_writer(stdout(), &SomeBody::from(&r))?,
    Format::Text => println!("..."),
}

// GOOD — the operation returns the typed body; the projector renders it
Ok(SomeBody::from(&result))
```

Text mode renders through the body's `emery_engine::handler::Render` impl (`fn render(&self, w: &mut dyn Write) -> io::Result<()>`); the JSON path goes through `serde::Serialize` automatically. New code must not introduce `match … format`.

## One emit path

Success bodies and failures leave operations as typed values. The command projector in `emery_engine::cli` renders those values at the command boundary; no handler writes stdout or stderr. If you need a bespoke failure shape, add an `Error` variant with a kebab-case discriminant; do not hand-roll a `*ErrBody` DTO. `emit` stays internal to `crates/engine/src/cli.rs`.

## DTOs

Response DTOs (`*Body`, `*Row`) are **top-level** structs under `mod`. Declaring a DTO inside a function body, match arm, or closure forces a per-file `#![allow(items_after_statements, …)]` suppression and is the signal that a handler hasn't been migrated yet.

**Construct DTOs through `From` impls, not named builders.** Use `impl From<&Domain> for Body` so the conversion is discoverable at the trait surface and call sites read `Body::from(&domain)`. Named constructors are reserved for multi-arg or fallible builders (e.g. `RegistryProposalRow::from_kind` returns `Option<Self>`); each survivor carries a one-line doc justification.

**Typed fields, not stringly-typed ones.** `pub status` / `pub kind` (and any other field whose domain has a finite enum) carry the underlying domain enum with `#[derive(Serialize)]` + `#[serde(rename_all = "kebab-case")]`. Drop `.to_string()` at construction sites; the wire shape is unchanged.

**`PathBuf` for path fields.** `*Body` fields that hold a filesystem path are `path: PathBuf`. Do not store `String` paths in DTOs; serde's default `PathBuf` serialization carries the bytes losslessly.

**Field-type allowlist.** DTO fields use the strictest type the wire shape supports:

| Domain | Type | Notes |
|---|---|---|
| Filesystem path | `PathBuf` | never `String`; serde's default carries the path losslessly |
| Status / kind / phase with finite domain | the underlying enum + `#[serde(rename_all = "kebab-case")]` | drop `.to_string()` at construction |
| Stable kebab discriminant | `&'static str` | lives in the binary |
| Timestamp written into JSON | `jiff::Timestamp` with the engine crate's `serde_time::rfc3339` adapter (or `rfc3339_opt` on `Option<Timestamp>`) | serde owns the format |
| Count | `usize` | JSON has neither `u32` nor `u64` |

**Single-variant enums are dead overhead.** Drop either the variant or the enum; the type's name already says "this DTO represents kind X". The `BriefAction::Init` pattern is the canonical example of what not to add.

```rust
// BAD — DTO inside fn body
fn handle(...) {
    #[derive(Serialize)]
    struct Body { name: String }
    output::write(format, &Body { name }, write_text)?;
}

// BAD — named builder, stringly-typed status, String path
impl Body {
    pub(crate) fn from_outcome(outcome: &Outcome, path: PathBuf) -> Self {
        Self {
            status: outcome.status.to_string(),
            path: path.display().to_string(),
        }
    }
}

// GOOD
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct HandleBody {
    name: String,
    status: OutcomeStatus,
    path: PathBuf,
}

impl Render for HandleBody {
    fn render(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(w, "{}", self.name)
    }
}

impl From<&Outcome> for HandleBody {
    fn from(outcome: &Outcome) -> Self { /* ... */ }
}
```

## Errors

Engine operations return `emery_engine::handler::Error`, an alias of `omnia_guest::Error` (`BadRequest`, `NotFound`, `ServerError`, `BadGateway`). Construct via `bad_request` / `not_found` / `server_error` / `bad_gateway` with a kebab `code` and a description. The kebab `code` is the public wire contract that skills and tests grep for; treat any rename as a breaking change.

**Class on a direct match.** Pick the Omnia variant that matches the failure: operator or input refusals are `BadRequest` (exit 1), missing resources are `NotFound` (exit 2), upstream or model failures are `BadGateway` (exit 4). Anything else — I/O, storage, leftover conversions — is `ServerError` (exit 3). Do not invent new kebab codes or new exit slots. See [handler-shape.md §"Exit codes"](./handler-shape.md#exit-codes).

**Hint lookup.** Long-form recovery hints live in `cli.rs` (`hint` on `adapter-cli-too-old` / `specify-source-required` / `spec-not-generated`). Adding a new hint extends that lookup, not the error type.

`unwrap()` and `expect()` are reserved for invariants the type system can't express (e.g. "this enum variant covers `Status::value_variants()`"). Always include a justification string in `expect`. User-facing errors must surface as an Omnia `Error` with a kebab `code`, not panics.

## `#[non_exhaustive]`

**Deliberate override of general library guidance, including the baseline's.** Public enums and structs are exhaustive by default: the workspace treats adding a variant as an ordinary pre-1.0 SemVer-minor event, and exhaustive matching at every consumer is the compile-time drift check the closed taxonomies (journal events, exit codes, lifecycle states) rely on. Reach for `#[non_exhaustive]` only when a type is genuinely open-ended *and* external consumers must keep compiling across additions; document that choice in a doc-line.

## JSON and storage

Structured interchange is JSON (`serde_json`). There is no live YAML path and no `Error::YamlDe` / `Error::YamlSer` variants.

Engine state rides the storage capabilities (`StateStore` / `BlobStore`), never tree writes — blobstore writes are complete-on-finalize, so no atomic-rename helper exists. `fs::write` is reserved for files outside engine state that no other live process reads (one-shot scratch output, fixtures inside a tempdir test).

## Module layout

Use the modern Rust module layout: `<parent>/<module>.rs` is the module entry point and child modules live under `<parent>/<module>/`. **Do not add `mod.rs` files** — `<module>/mod.rs` is the legacy 2018-edition pattern and is forbidden in workspace crates. The single allowed exception is `tests/<helper>/mod.rs`, which is the documented Rust idiom for sharing code between integration test binaries (`tests/<helper>.rs` would be picked up as its own test target). When you split a file, create `<module>.rs` + `<module>/<concern>.rs`; never reach for `<module>/mod.rs`.

```text
crates/foo/src/
├── widget.rs            ← module entry (was widget/mod.rs)
└── widget/
    ├── parse.rs
    └── render.rs
```

**Module length cap** — keep new modules ≤ 400 lines. When a file outgrows that, split by concern (one verb per file, model vs IO vs transitions, etc.) before adding more code. Prefer `<parent>/<module>.rs` + `<parent>/<module>/<concern>.rs` over a single fat file with `// ---` separators.

## No-op forwarders

A clap-parsed flag that is destructured and silently dropped (`let _ = cli.<flag>;` or pattern matches that never reach a handler) is a YAGNI smell. Either the flag is wired up (the variant carries data and the handler reads it) or it is removed from clap.

## Wired-but-ignored flags

A flag whose doc-comment says "Currently equivalent to the default …" or whose handler ignores the value is the same defect as `no-op-forwarders` dressed up as documentation. Drop the flag from clap until the differentiated behaviour exists.

## Drift audit

When you remove a symbol, run `rg <SymbolName> -- AGENTS.md docs/` and update every hit in the same PR. Stale symbol references in docs are worse than missing docs — they teach the reader something false. Doc drift on internal symbols (error variants, type names, field keys) is caught only by this audit habit.
