# Coding standards

Style rules every Rust change in this workspace honours. Enforced by clippy (`cargo make lint`) and review. When a rule fights you, add the case to the rule with a before/after — don't carve out a local exception.

## Lints

Workspace lints live in `Cargo.toml`. Defaults are aggressive — clippy `all`/`cargo`/`nursery`/`pedantic` are all `warn`, plus a curated set of `restriction` lints and a tightened rust lint set (`missing_debug_implementations`, `single_use_lifetimes`, `redundant_lifetimes`). Compile under `RUSTFLAGS=-Dwarnings` (`cargo make test` does this), so any new warning fails CI.

Visibility on internal items follows clippy's `redundant_pub_crate` (nursery) rather than rustc's `unreachable_pub`: prefer bare `pub` and let the parent module's privacy do the constraining. The two lints are mutually exclusive — enabling both would loop. `unreachable_pub` stays at its allow-by-default, and any `#[expect(unreachable_pub, …)]` carve-out is a rot signal, not a tool you reach for.

When you must silence a lint, use `#[expect(<lint>, reason = "…")]` at the **smallest possible scope**. `#[expect]` is preferred over `#[allow]` everywhere except module-level waivers: a dead `#[expect]` is a build failure, so the suppression cannot rot. `#![allow(...)]` at the crate or module root is still the right tool when the lint legitimately applies to every item below. Doc idents such as `GitHub`, `MiB`, `OAuth`, `OpenTelemetry`, `SemVer`, `WebAssembly`, and `YAML` live in `clippy.toml` `doc-valid-idents`.

`taplo.toml` formats `Cargo.toml` files. Dependency arrays under `*-dependencies` and `dependencies` reorder alphabetically; preserve that on edit.

## Lint suppression posture

Site-local suppressions are `#[expect(<lint>, reason = "…")]`, not `#[allow]` — a dead `#[expect]` is a build failure, so the suppression cannot rot. Module-level waivers stay `#![allow(<lint>, reason = "…")]` because lint-rot detection at the module root is not useful (the waiver typically covers many sites). Identical `reason = "…"` strings across three or more files mean you should promote a single `#![allow]` to the parent module — the file-level repetition is noise, not signal.

```rust
// BAD — site-local #[allow]
#[allow(clippy::cognitive_complexity, reason = "linear state machine")]
fn step(...) { ... }

// GOOD — same scope, #[expect]
#[expect(clippy::cognitive_complexity, reason = "linear state machine")]
fn step(...) { ... }

// GOOD — module-root waiver that legitimately covers every item below
// crates/project/src/agents.rs
#![allow(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    reason = "binary-internal context-fence code consumed only by the `agents` command; documenting ~30 internal fields adds noise, not API surface"
)]
```

## Comments

Comments answer "why does this look like this *today*?" — non-obvious intent, trade-offs, or constraints the code itself can't convey. Migration trails, old labels, and "this used to be X" rationale belong in commit messages — not in code or doc comments. Doc comments on items that surface in `--help` (clap `#[derive]` fields) must be operator-facing one-liners; rationale moves below the derive block where it doesn't leak into help output.

```rust
// BAD
//! Per the workspace split 2.9 ("Init wires components, not adapters"),
//! `init` writes only the per-project skeleton — `project.yaml` plus
//! the `.specify/` tree. The pre-Phase-3.7 filename was `charter.md`;
//! Historical rename detail belongs in git history, not module docs.

// GOOD
//! Scaffolds `.specify/` plus `project.yaml`. Operator-facing artifacts
//! (`registry.yaml`, `change.md`, `plan.yaml`) are minted by their
//! owning verbs, not by `init`.
```

Doc comments describe what this is today. Version-history tables, dated bumps, commit hashes, and migration notes belong in git log — not in `///` blocks. Keep `///` paragraphs on `pub` items under ~8 lines; longer prose belongs in the standards docs.

`cargo doc` is part of `cargo make ci`, so doc comments must compile. Reference paths inside backticks (`` `Self::config_path` ``) are fine; bare links (`[Foo]`) need a corresponding intra-doc target or rustdoc fails the build.

## Naming

Prefer short, idiomatic Rust names. Don't restate context the surrounding module, type, or function already supplies. Avoid `_local` / `_value` / `_helper` suffixes. New functions: 1–3 words. Predicates start with `is_` / `has_`. DTOs returned by handlers are `<Action>Body` / `<Action>Row`, never `<Action>Response` / `<Action>Json` (the type's role is `Body`; the format dispatch lives in `emit` — see [handler-shape.md](./handler-shape.md)).

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
- **Comment redundancy**: don't paraphrase a `match` arm's variant in a `// …` comment when the variant's doc-comment already explains it. The same rule applies to `Exit::code()`'s inline comments mirroring variant docs.

`verbose-doc-paragraphs` (8-line cap on `pub` items) and `ritual-doc-paragraphs` (boilerplate "Returns an error if …") catch the mechanical cases. Brevity at the type, field, and variant level is on you.

## Format dispatch

Operations do **not** open-code `match format { Json, Text }`. They return typed bodies; the command projector (`SpecifyProjector` in `crates/transport/src/command.rs`) owns format dispatch through the internal `emit` function in `crates/transport/src/command/output.rs`. Operations never pick a sink directly. See [handler-shape.md](./handler-shape.md) for the operation and projector contract.

```rust
// BAD
match format {
    Format::Json => serde_json::to_writer(stdout(), &SomeBody::from(&r))?,
    Format::Text => println!("..."),
}

// GOOD — the operation returns the typed body; the projector renders it
Ok(SomeBody::from(&result))
```

Text mode renders through the body's `project::handler::Render` impl (`fn render(&self, w: &mut dyn Write) -> io::Result<()>`); the JSON path goes through `serde::Serialize` automatically. New code must not introduce `match … format`.

## One emit path

Success bodies and failures leave operations as typed values. The projectors in `crates/transport` render those values at the command or HTTP boundary; no handler writes stdout or stderr. If you need a bespoke failure shape, add an `Error` variant with a kebab-case discriminant; do not hand-roll a `*ErrBody` DTO. `emit` stays internal to `crates/transport/src/command/output.rs`.

## DTOs

Response DTOs (`*Body`, `*Row`) are **top-level** structs under `mod`. Declaring a DTO inside a function body, match arm, or closure forces a per-file `#![allow(items_after_statements, …)]` waiver and is the signal that a handler hasn't been migrated yet.

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

`error::Error` variants are **structured**, not `Variant(String)` catch-alls. The kebab-case identifier in `#[error("…")]` (and in `Error::Diag.code`) is part of the public contract that skills and tests grep for; treat any rename as a breaking change.

**Diag-first error policy.** New diagnostic sites use `Error::Diag { code: "<kebab>", detail: format!(…) }`. Promote to a typed `Error::*` variant **only** when:

1. A test or skill destructures the variant's payload, **or**
2. The variant routes to a non-default `Exit` slot (validation / argument / version-too-old — see [handler-shape.md §"Exit codes"](./handler-shape.md#exit-codes)), **or**
3. Three or more call sites share the variant's exact shape.

The kebab `code` is the wire contract; the Rust variant is for callers that pattern-match. Adding a typed variant for a one-site diagnostic doubles the `variant_str` table for no functional gain. When in doubt, stay on `Diag`.

A dedicated typed variant remains correct for entries that already meet the criteria above (`Error::Argument`, `Error::Validation`, `Error::Filesystem`, …). Typed variants must carry data the *wire* exposes; payloads that only round-trip into `detail` belong on `Error::Diag`.

**Hint colocation.** Long-form recovery hints live on the error, not on the renderer. `Error::hint(&self) -> Option<&'static str>` is the single hint surface; `ErrorBody::render_text` calls it. Adding a new hint means extending `Error::hint`, not the renderer.

`unwrap()` and `expect()` are reserved for invariants the type system can't express (e.g. "this enum variant covers `Status::value_variants()`"). Always include a justification string in `expect`. User-facing errors must surface as `Error::*` variants, not panics.

## `#[non_exhaustive]`

Every public `enum` or `struct` that may grow gets `#[non_exhaustive]`. The exception is structurally complete types (`enum Format { Json, Text }`); document the choice in a doc-line. This keeps adding a variant from being a SemVer break.

## YAML, JSON, and atomic writes

YAML (de)serialization goes through `serde-saphyr`, not `serde_yaml_ng` or the deprecated `serde_yaml`. `serde-saphyr` has no `Value` type; for dynamic YAML access deserialize into `serde_json::Value`. Deser and ser errors ride directly on `error::Error::YamlDe(serde_saphyr::Error)` and `Error::YamlSer(serde_saphyr::ser::Error)` — both `#[error(transparent)]` `#[from]` variants — so `?` on a raw `serde_saphyr` result still propagates, the kebab discriminant on the wire stays `yaml` for either side, and call sites that don't care which API tripped match on either variant. Library crates return `Result<…, error::Error>` rather than re-exposing `serde_saphyr::*::Error` types in their own public signatures.

Writes that must not be observed mid-update use the shared atomic helpers in `specify_slice::atomic` (`yaml_write` / `bytes_write`). `fs::write` is fine for single-shot scratch files but never for files that other live processes read (`plan.yaml`, `registry.yaml`, `change.md`, `tasks.md`, `metadata.yaml`). See [architecture.md §"Atomic writes"](./architecture.md#atomic-writes) for the rationale.

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
