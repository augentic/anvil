# RFC-44: Authoring vs Runtime Surface Split (`specdev` / `specrun`)

> Status: Draft · Relates: [RFC-43 authoring configuration](./rfc-43-authoring-config.md) (consumed by this RFC's auto-discovery step), [RFC-41 version binding](./rfc-41-version-binding.md) (the Rust-free authoring path this RFC must preserve), [Roadmap §"Keep enforcement surfaces distinct"](./roadmap.md) (`specdev lint` / `specrun lint`) · Scope: `augentic/specify-cli` (the binary surface); on acceptance the standing decision graduates into `specify-cli` [`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md)

## Abstract

One `specify` binary serves two audiences whose needs diverge: **operators** running the consumer-project runtime (`plan`, `slice`, `workspace`, `init`, `migrate`, `lint project`) and **framework authors** running authoring checks (`lint framework`, acceptance frontmatter validation). The surfaces are blurred under one verb set, and the roadmap already reserves two distinct names — `specdev` (authoring) and `specrun` (runtime) — without a design for how they relate.

This RFC proposes giving the two surfaces crisp, discoverable identities **without forking the binary**: ship one build artifact and dispatch on `argv[0]`, installed under three names (`specify`, `specrun`, `specdev`). It also lets the binary auto-discover the [RFC-43](./rfc-43-authoring-config.md) authoring config so `scripts/specify.sh` can shrink to a bootstrap shim. A genuine two-binary split is analysed and **reserved** as a deferred section — the crate graph already supports it cleanly — gated on a concrete forcing function rather than landed speculatively.

## Motivation

### The roadmap already named the split; the design is missing

[`roadmap.md`](./roadmap.md) reserves separate enforcement surfaces — `specdev lint` for framework-repo authoring standards and `specrun lint` for consumer-project engineering standards — and spells every mid/long-term target surface `specrun …`. The intent is settled; what is missing is *how* the two names relate to the one binary that exists today, and whether they are one artifact or two.

### One binary, two audiences, blurred surface

The `Commands` enum in [`src/runtime/cli.rs`](https://github.com/augentic/specify-cli/blob/main/src/runtime/cli.rs) mixes runtime verbs (`Source`, `Target`, `Slice`, `Plan`, `Workspace`, `Init`, `Migrate`, `Upgrade`) with the authoring-only `Lint { Framework }` sitting beside the runtime-only `Lint { Project }`. An operator's `--help` lists framework-authoring surface they will never use; an author's lists workflow lifecycle verbs they will never use. The split is real — `docs/explanation/standards-layer.md` enforces it as a *type-system* invariant (`specify-standards` ⟂ `specify-workflow`) — but the CLI surface does not reflect it.

### The authoring audience is bimodal — and one half must stay Rust-free

This is the load-bearing constraint. [`docs/contributing/index.md`](../docs/contributing/index.md) documents two authoring audiences with opposite toolchain needs:

| Audience | Edits | Rust/Cargo needed? |
| --- | --- | --- |
| **Skill / adapter / rule authors** | `SKILL.md`, briefs, references, rules, docs | **No** — markdown / YAML only |
| **Toolsmiths / CLI devs** | `specify-standards` predicates, schemas, WASI tools, generated-crate codegen | **Yes** |

RFC-41's entire purpose was to make the **first** group Rust-free: `make lint` runs `lint framework` from a published binary with no `specify-cli` checkout and no Rust toolchain. Therefore **the authoring surface (`specdev`) must remain a standalone, Rust-free binary**, identical in install posture to the runtime surface. Any design that makes authoring a Cargo extension would regress RFC-41 for the majority authoring audience.

The work that *does* require Cargo — the `specify-standards` predicate regression suite, `cargo make framework-wasm` (building the embedded WASI checkers), and the RFC-42 Phase-2 generated-output gates (`cargo check` / `cargo test` on generated crates) — **already lives in the `specify-cli` workspace and is already driven by `cargo make`** (see [`Makefile.toml`](https://github.com/augentic/specify-cli/blob/main/Makefile.toml)). That is the Cargo-native authoring tier; it does not belong on `specdev`, and `specdev` must not require it.

So the honest picture is three tiers, only the third of which is Cargo-bound:

| Tier | Surface | Config | Rust? |
| --- | --- | --- | --- |
| **1. Runtime** | `specrun` (`plan`, `slice`, `workspace`, `init`, `migrate`, `lint project`) | `.specify/project.yaml` | No |
| **2. Framework authoring** | `specdev` (`lint framework`, acceptance frontmatter) | `Specify.toml` (RFC-43) | No |
| **3. Toolsmith / CLI dev** | `cargo make` in `specify-cli` | `Makefile.toml` | Yes |

Tiers 1 and 2 are the *same binary* under two names. Tier 3 is the existing Cargo workflow and is out of scope for the binary surface.

## Design: one binary, `argv[0]` dispatch, three names

Ship a single build artifact. The binary inspects `argv[0]` (the install name) and selects the surface it presents:

- invoked as **`specrun`** → only Tier-1 runtime verbs are registered.
- invoked as **`specdev`** → only Tier-2 authoring verbs are registered.
- invoked as **`specify`** → the full union (back-compat; the name in every doc and skill brief today).

Installation places `specify` and adds `specrun` / `specdev` as hardlinks or symlinks (Homebrew formula, `install.sh`, and `cargo install` post-step all create the three names). This is the BusyBox / `cargo`-and-`rustc`-share-infrastructure pattern: the "two components" the operator and author perceive are a *packaging* detail, not two builds.

### Why one binary beats two

| Property | One binary + `argv[0]` | Two separate binaries |
| --- | --- | --- |
| Release pipelines | One | Two |
| Version-compat matrix | One `[cli]` binding (`RFC-43` `version` / `binary`) | Two pins + a matrix |
| Shared `Ctx` / `Out` / dispatch / exit-code map | Reused | Duplicated or extracted to a shared crate |
| Embedded WASI framework checkers | Carried once | Must be excluded from `specrun`, carried by `specdev` |
| RFC-41 Rust-free authoring | Preserved trivially | Preserved only if `specdev` is also a published standalone binary |
| Operator-perceived surface | Clean (`specrun --help`) | Clean |
| Author-perceived surface | Clean (`specdev --help`) | Clean |

The one-binary form delivers the *entire* ergonomic and conceptual benefit (clean per-audience `--help`, the roadmap's names, surface separation) at none of the dual-pipeline cost.

### Clap surface

`argv[0]` is read before clap parses, and the `Commands` subcommand set is built conditionally. Concretely: factor `Commands` into `RuntimeCommands` and `AuthoringCommands` (the existing `Lint { Project }` vs `Lint { Framework }` already foreshadows the seam), and assemble the active `clap::Command` per the resolved surface. `completions` is generated per surface so `specrun`/`specdev` shell completions list only their own verbs. The global `--format` flag and the exit-code contract (`Exit::from(&Error)` in `src/runtime/output.rs`) are unchanged.

### Consuming the RFC-43 authoring config

When invoked as `specdev` (or `specify` in an authoring-repo context), the binary auto-discovers `Specify.toml` at the repo root and reads `cli.version`, `cli.binary`, and optional `cli.path` as defaults — explicit flags and `SPECIFY_VERSION` still override. The authoring lint scan root is not read from the file: `specify lint framework` keeps `--framework-root` (default `.`), and auto-discovery may later infer the root from the directory containing `Specify.toml`. This is the step that lets [`scripts/specify.sh`](../scripts/specify.sh) shrink: once the binary reads the file itself, the script's responsibility collapses to materializing into `cli.binary` and exec'ing it, with the declared values living in TOML rather than duplicated in Bash.

### Migration

Additive, no behaviour removed:

- Factor `Commands` into runtime / authoring subsets in `src/runtime/cli.rs`; keep `specify` presenting the union.
- Add `argv[0]` resolution at binary entry; register the subset for `specrun` / `specdev`.
- Update `install.sh`, the Homebrew formula, and the `cargo install` docs to create all three names.
- Add `specdev` auto-discovery of `Specify.toml` (depends on [RFC-43](./rfc-43-authoring-config.md) having defined the file).
- Update `docs/contributing/`, `AGENTS.md`, and skill briefs that say `specify lint framework` to also recognise `specdev lint framework`; the `specify`-prefixed forms keep working.

Order: RFC-43 defines the file; RFC-44 consumes it. RFC-44's `argv[0]` and clap-factoring work has no dependency on RFC-43 and can land first; only the auto-discovery step waits on it.

## Reserved: the hard two-binary split (deferred)

This RFC deliberately does **not** split the binary into two build artifacts. It records the cut so a future RFC can execute it cheaply when a trigger fires.

**The cut is already a linker decision, not a refactor.** Per [`docs/explanation/standards-layer.md`](../docs/explanation/standards-layer.md) and `specify-cli`'s `AGENTS.md`, `specify-standards` (authoring/standards) and `specify-workflow` (runtime) are siblings — neither imports the other, and "the only place both crates meet is the root `specify` binary." A hard split therefore builds two root binaries:

- **`specrun`** — links `specify-workflow` + `specify-standards` (for `lint project`) + `specify-{model,schema,diagnostics}`. No embedded framework WASI checkers.
- **`specdev`** — links `specify-standards` + `specify-schema` + the framework `Check` / lint dispatcher + the embedded framework WASI checkers. Still a standalone, Rust-free published binary (RFC-41 posture preserved).

**Triggers that would justify paying for it:**

- The embedded framework WASI checkers (or authoring-only schemas) materially bloat the operator runtime install.
- Runtime and authoring acquire genuinely incompatible dependency or release cadences.
- A security or supply-chain boundary requires the operator binary to carry strictly less code.

**Costs to budget when the trigger fires:** two release pipelines, two version pins with a compatibility matrix, and ensuring the embedded WASI checkers land in `specdev` only. Until then, the `argv[0]` form above gives the same surface separation at single-pipeline cost.

## Non-Goals

- **No Cargo-gated authoring surface.** `specdev` stays a standalone, Rust-free published binary; the Cargo-bound work stays in Tier 3 (`cargo make`). This RFC explicitly rejects making authoring a `cargo` subcommand, which would regress [RFC-41](./rfc-41-version-binding.md).
- **No two build artifacts now.** The hard split is reserved, not landed.
- **No change to runtime config or authoring config shape.** `.specify/project.yaml` is untouched; `Specify.toml` is defined by [RFC-43](./rfc-43-authoring-config.md), consumed here.
- **No new lifecycle or gate authority.** Renaming surfaces does not move the lint/validate authority boundary; `lint` stays lifecycle-neutral and silenceable, `validate` stays gating.
- **No removal of the `specify` name.** `specify` remains the union surface for back-compat with every existing doc, skill brief, and `scripts/specify.sh` passthrough.

## Open Questions

1. **Default surface for a bare `specify`.** Keep the full union (chosen here), or have `specify` print a chooser/deprecation nudge toward `specrun` / `specdev` over time?
2. **Where `specdev` looks for `Specify.toml`.** Nearest-ancestor walk (like `lint project`'s `.specify/project.yaml` resolution) vs. require it at the invocation root.
3. **Completion packaging.** Three completion scripts (one per name) vs. one script that detects the invoked name.
4. **Decision home.** Does the accepted `argv[0]` design graduate into `specify-cli` `DECISIONS.md` (consistent with how that repo records standing decisions), with this RFC retained as the rationale, or stay solely as an RFC?

## References

- [`rfc-43-authoring-config.md`](./rfc-43-authoring-config.md) — the authoring config this RFC consumes.
- [`rfc-41-version-binding.md`](./rfc-41-version-binding.md) — the Rust-free authoring path this RFC must preserve.
- [`roadmap.md`](./roadmap.md) — the reserved `specdev` / `specrun` names and the `specrun …` target surfaces.
- [`docs/contributing/index.md`](../docs/contributing/index.md) — the bimodal authoring audience (Rust-required vs not).
- [`docs/explanation/standards-layer.md`](../docs/explanation/standards-layer.md) — the `specify-standards` ⟂ `specify-workflow` type-system split the hard cut would follow.
- [`docs/contributing/checks.md`](../docs/contributing/checks.md) — `lint framework` vs `lint project` and the embedded framework WASI checkers.
- [`specify-cli` `src/runtime/cli.rs`](https://github.com/augentic/specify-cli/blob/main/src/runtime/cli.rs) — the `Commands` enum to factor into runtime / authoring subsets.
- [`specify-cli` `Makefile.toml`](https://github.com/augentic/specify-cli/blob/main/Makefile.toml) — the Tier-3 `cargo make` surface this RFC leaves untouched.
- [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) — the standing-decision home the accepted design graduates into.
