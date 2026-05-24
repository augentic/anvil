# RFC-5: Framework Linter

> Status: Draft · Tracked by [roadmap RM-16](roadmap.md#rm-16-rfc-5-specify-check-framework-linter-port) · Enables: [RFC-4](rfc-4-dsl.md)

## Abstract

Port the repo's existing `scripts/checks.ts` Deno framework linter into a Rust `specify-check` crate exposed via `specify check`. The port is a message-preserving one-for-one migration — each Deno module under `scripts/checks/` maps to a Rust module with identical semantics — that runs alongside the Deno script during rollout and retires it once parity is reached.

## Motivation

`scripts/checks.ts` is the framework-level linter for this repo. It runs in CI and enforces invariants the runtime CLI does not need to know about: adapter manifest conformance against `source.schema.json` / `target.schema.json`, brief size and brief-no-frontmatter discipline, marketplace.json alignment, SKILL.md frontmatter and body discipline, cross-skill directive validity, codex rule shape, declared-tool invocation equivalence, scenario fixtures, and docs hygiene.

The script works. At ~3 200 lines across 13 modules under [`scripts/checks/`](../scripts/checks/) it is not broken, not blocking other RFCs, and not on the critical path for the `specify` CLI's runtime surface. But the port is still worth doing:

- It removes the Deno toolchain from `make checks` and collapses the CI dependency surface onto `cargo`.
- It lets the linter share `specify-domain`'s parsers (adapter manifests, brief paths, codex rules) instead of maintaining a parallel YAML pipeline in TypeScript.
- It gives [RFC-4](rfc-4-dsl.md)'s Option 1 (CLI-integrated skill validation) a home that already understands the repo's adapter and skill model.

## Detailed Design

### Scope

`specify-check` ports every invariant currently enforced by `scripts/checks.ts` and its modules under `scripts/checks/`, and nothing else. New invariants belong to follow-up RFCs or to RFC-4's later options.

The boundary against the runtime crates is unchanged: the `specify` CLI's `specify-domain` / `specify-tool` / `specify-error` stack validates *consumer projects* (artifact correctness at runtime, adapter manifest loads, slice lifecycle transitions); `specify-check` validates *the plugin repo* (skill integrity, adapter brief discipline, marketplace alignment, docs hygiene at CI time). The overlap is intentional and narrow: both parse `adapter.yaml`, so `specify-check` depends on `specify-domain` for that parser and for the per-axis schemas shipped via `include_str!`. Everything else (symlink resolution, SKILL.md frontmatter, brief size, scenario fixtures, marketplace) is plugin-repo-specific and lives in `specify-check`.

### Workspace layout

`specify-check` is added as a peer leaf to `specify-tool` in the existing `specify-cli` workspace (per the leaf → root graph documented in the CLI's [AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md#crate-graph)):

```text
specify-cli/
├── Cargo.toml                          # workspace manifest + root `specify` package
├── src/                                # binary + top-level lib
├── crates/
│   ├── error/                          # specify-error — leaf
│   ├── tool/                           # specify-tool — depends on specify-error
│   ├── domain/                         # specify-domain — depends on {error, tool}
│   └── check/                          # specify-check — this RFC; depends on {error, domain}
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── adapter.rs              # adapter.yaml ↔ source/target schemas
│           ├── brief.rs                # brief size + no-frontmatter discipline
│           ├── codex.rs                # codex rule shape
│           ├── docs_quality.rs         # RFC citation hygiene, diagram assets
│           ├── links.rs                # markdown links + cross-skill directives + references
│           ├── plugins.rs              # symlinks + marketplace.json consistency
│           ├── prose.rs                # invocation positionals, operational vocab, numeric caps
│           ├── scenarios.rs            # scenario frontmatter + recorded-trace freshness
│           ├── skill_body.rs           # skill body discipline (12 predicates)
│           ├── skill_frontmatter.rs    # skill frontmatter discipline (7 predicates)
│           └── tools.rs                # declared-tool equivalence + first-party tool decls
```

The augentic/specify repo continues to invoke the linter from `make checks`, which calls `specify check --repo .`. Until parity is reached the Deno script keeps running side-by-side.

### Module design

Each Rust module ports the corresponding Deno module under [`scripts/checks/`](../scripts/checks/) so parity can be verified module-by-module. Failure messages MUST match the current `checks.ts` wording so CI diffs stay readable while both tools run side-by-side; message stability is how we know the migration is safe to finish removing `checks.ts`.

The 12 modules group into five concerns:

| Concern               | Deno module(s)                                            | Rust module          | Notes                                                                                                              |
|-----------------------|-----------------------------------------------------------|----------------------|--------------------------------------------------------------------------------------------------------------------|
| Adapter manifests     | `adapter.ts`                                              | `adapter.rs`         | Validates each `adapters/{sources,targets}/<name>/adapter.yaml` against the schemas shipped by `specify-domain` via `include_str!`. |
| Brief discipline      | `brief_size.ts`                                           | `brief.rs`           | Walks `adapters/{sources,targets}/<axis>/<name>/briefs/**/*.md`. Enforces parent ≤ 150 / phase ≤ 800 non-blank lines and **no YAML frontmatter on any brief**. Briefs are not skills — they are resolved by path from `adapter.yaml`. |
| Skill discipline      | `skill_frontmatter.ts`, `skill_body.ts`, `prose.ts`       | three peer modules   | Description grammar, argument-hint grammar, 200/45/512 caps, body restatement, no-RFC-citations-in-bodies, invocation positionals, operational vocab, numeric-cap sync. |
| Repo hygiene          | `links.ts`, `plugins.ts`, `docs_quality.ts`               | three peer modules   | Markdown link resolution (including symlink-aware references under `plugins/spec/references/`), `.cursor-plugin/marketplace.json` ↔ plugin layout consistency, RFC-citation hygiene in `docs/`, diagram assets. |
| Specialist surfaces   | `codex.ts`, `scenarios.ts`, `tools.ts`                    | three peer modules   | Codex rule shape against `.cursor/schemas/codex-rule.schema.json`, scenario frontmatter against `scenario.schema.json` plus recorded-trace freshness, declared-tool equivalence between `tools.yaml` and the `specify tool run` invocations referenced in skill bodies. |

`links.rs` in particular carries load-bearing symlink behaviour (the existing `underSymlink` gate in `_shared.ts` governs how shared reference docs under `plugins/spec/references/` are traversed). The port MUST preserve it.

### `specify check` subcommand

The root `specify` package (`src/main.rs`) gains a single new subcommand:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing subcommands ...

    /// Validate the specify framework / plugin repo itself
    Check {
        /// Repository root (defaults to current directory)
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
}
```

The subcommand is a thin dispatcher that runs every `specify-check` module concurrently (mirroring the `Promise.all` batches in `scripts/checks.ts`) and aggregates results into the JSON envelope shape `specify` uses for its other commands (see the CLI repo's [output shape contract](https://github.com/augentic/specify-cli/blob/main/docs/standards/handler-shape.md)). Exit code follows the standard exit-code table — `0` on success, `2` on validation failures, `1` on infrastructure errors (I/O, schema load failures).

### Dependencies

```toml
# crates/check/Cargo.toml
[dependencies]
specify-domain = { path = "../domain" }
specify-error  = { path = "../error" }
serde         = { workspace = true, features = ["derive"] }
serde_json    = { workspace = true }
serde-saphyr  = { workspace = true }
jsonschema    = "0.29"
```

`jsonschema` pulls in `fancy-regex`, `ahash`, `url`, and a handful of transitive crates. This is acceptable because `specify-check` is CI-only — none of that weight leaks into the runtime crates (`specify-domain` ships its own schemas via `include_str!` and validates with `jsonschema::Validator` already, so the dependency is already in the graph for the runtime build).

### Migration strategy

The port is a rolling migration, not a flag day:

1. **Land `specify-check` as a stub.** `specify check` wires up the CLI subcommand, depends on `specify-domain` and `specify-error`, and has empty-but-compiling modules. `make checks` runs both tools; both pass trivially. This change is mechanical and self-contained.
2. **Port `adapter` and `brief` first.** They are the modules that share the most with `specify-domain` (the per-axis manifest schemas, the adapter brief-path conventions). Porting them first validates the parser reuse story before investing in the larger skill modules.
3. **Port the remaining modules one at a time.** Each port deletes the corresponding function (or block) from `scripts/checks/<module>.ts`. Failure messages MUST match during the overlap period so CI diffs stay readable.
4. **Retire `scripts/checks.ts`.** When the orchestrator and every module under `scripts/checks/` are empty, delete the tree, remove the Deno dependency from the Makefile, and switch CI to `specify check` only.

Each step is independently mergeable and leaves CI green.

### Makefile integration

```makefile
.PHONY: checks
checks:
	specify check --repo .
	deno run --allow-read scripts/checks.ts  # keep during migration; remove once empty
```

The second line is removed once the Deno tree is gone; until then both tools run side-by-side and any discrepancy between them is treated as a port regression.

## Alternatives Considered

**Keep `scripts/checks.ts` indefinitely.** Rejected because maintaining a second toolchain (Deno) for the framework linter adds CI friction and duplicates the adapter-manifest / brief-path parsing logic `specify-domain` already owns. The duplication is small today, but every new schema-level invariant doubles the implementation work until the port is done.

**Rewrite the linter from scratch.** Skips the fidelity constraints but throws away the invariants the current script already encodes. Those invariants capture real lessons about repo drift; preserving them verbatim is cheaper than re-deriving them, and the message-preserving migration lets CI act as a regression test for the port itself.

**Land the linter outside the `specify-cli` workspace.** Considered separating the linter into its own crate or repo so the runtime CLI stays narrower. Rejected because the linter's largest dependency (`specify-domain` for adapter parsing and the bundled schemas) is already in the CLI workspace; isolating the linter would require either duplicating the parser or publishing `specify-domain` as a crates.io crate purely to satisfy the linter.

## References

- [`scripts/checks.ts`](../scripts/checks.ts) + [`scripts/checks/`](../scripts/checks/) — the modules being ported.
- [`docs/standards/skill-authoring.md`](../docs/standards/skill-authoring.md) — the invariants the skill-discipline modules enforce.
- [`docs/explanation/adapter-anatomy.md`](../docs/explanation/adapter-anatomy.md) — the RFC-25 adapter model the brief and adapter modules validate.
- [Specify CLI `AGENTS.md`](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) — the crate graph this RFC slots into.
- [RFC-4: Type-Safe Skill Expression](rfc-4-dsl.md) — Option 1 is satisfied once this port is complete.
