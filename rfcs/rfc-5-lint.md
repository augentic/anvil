# RFC-5: Framework Linter

> Status: Draft · Depends: [RFC-1](archive/rfc-1-cli.md) · Enables: [RFC-4](rfc-4-dsl.md)

## Abstract

Port the repo's existing `scripts/checks.ts` Deno framework linter into a Rust `specify-check` crate exposed via `specify check`. The port is a message-preserving one-for-one migration — each TypeScript section in `checks.ts` maps to a Rust module with identical semantics — that runs alongside the Deno script during rollout and retires it once parity is reached.

## Motivation

`checks.ts` is the framework-level linter for this repo. It runs in CI and enforces invariants the runtime CLI does not need to know about: `schema.yaml` ↔ JSON Schema conformance, brief frontmatter integrity, marketplace.json alignment, SKILL.md reference resolution, cross-skill directive validity, and `docs/plugins.md` inventory coverage.

The script works. At ~650 lines across 11 modules it is not broken, not blocking RFC-2/RFC-3/RFC-4, and not on the critical path for the `specify` CLI's runtime surface. Earlier drafts of [RFC-1](archive/rfc-1-cli.md) bundled the port into Phase 1; that bundling roughly doubled Phase 1's scope without improving the runtime story downstream skills consume. Separating the port into its own RFC lets RFC-1 ship the runtime CLI quickly and lets the `checks.ts` migration proceed as a focused, message-preserving rollout once the workspace foundation is in place.

The port itself is still worth doing:

- It removes the Deno toolchain from `make checks` and collapses the CI dependency surface onto `cargo`.
- It lets the linter share `specify-schema`'s parsers (schema + brief frontmatter) instead of maintaining a parallel YAML pipeline in TypeScript.
- It gives [RFC-4](rfc-4-dsl.md)'s Option 1 (CLI-integrated skill validation) a home that already understands the repo's schema and skill model.

## Detailed Design

### Scope

`specify-check` ports every invariant currently enforced by `scripts/checks.ts` and nothing else. New invariants belong to follow-up RFCs or to RFC-4's later options.

The boundary against the runtime crates is unchanged from RFC-1: the `specify-validate` / `specify-spec` / `specify-schema` stack validates *consumer projects* (artifact correctness at runtime); `specify-check` validates *this repo* (skill integrity, schema consistency, marketplace alignment at CI time). The overlap is intentional and narrow: both parse `schema.yaml` and brief frontmatter, so `specify-check` depends on `specify-schema` for those parsers. Everything else (symlink resolution, SKILL.md frontmatter, docs inventory) is repo-specific and lives in `specify-check`.

### Workspace Layout

`specify-check` is added as a third crate to the workspace defined by [RFC-1](archive/rfc-1-cli.md):

```
specify/
├── Cargo.toml                        # workspace manifest + root `specify` package (RFC-1)
├── src/                              # RFC-1 — binary + top-level lib
├── crates/
│   ├── specify-error/                # RFC-1
│   ├── specify-schema/               # RFC-1
│   ├── specify-spec/                 # RFC-1
│   ├── specify-merge/                # RFC-1
│   ├── specify-task/                 # RFC-1
│   ├── specify-validate/             # RFC-1
│   ├── specify-change/               # RFC-1
│   ├── specify-drift/                # RFC-1
│   ├── specify-platform/             # RFC-1
│   └── specify-check/                # this RFC — framework validation (replaces checks.ts)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── links.rs              # markdown link resolution
│           ├── schema_structure.rs   # schema.yaml validation against JSON Schema
│           ├── schema_integrity.rs   # pipeline/brief/needs/id referential integrity
│           ├── skill_frontmatter.rs  # SKILL.md frontmatter validation
│           ├── skill_references.rs   # skill reference/symlink resolution
│           ├── skill_variables.rs    # variable consistency
│           ├── skill_directives.rs   # <!-- skill: plugin:skill --> validation
│           ├── marketplace.rs        # marketplace.json vs plugin.json consistency
│           └── plugins_doc.rs        # docs/plugins.md inventory check
└── scripts/                          # existing — checks.ts stays during migration
```

### Module Design

Each module maps to a discrete section of the current script so parity can be verified module-by-module. Failure messages MUST match the current `checks.ts` wording so CI diffs stay readable while both tools run side-by-side; message stability is how we know the migration is safe to finish removing `checks.ts`.

#### `schema_structure.rs`

Thin wrapper around `jsonschema` that validates every `schemas/*/schema.yaml` against `.cursor/schemas/specify-schema.schema.json`. This replaces `checks.ts`'s `validateSchemaYaml` (lines ~125-147) and produces one failure per JSON-Schema error with `instancePath` and message preserved in the output.

#### `schema_integrity.rs`

Replaces `checks.ts`'s `checkSchemaIntegrity` (lines ~149–270). The port MUST preserve the real schema shape — a `pipeline` of `{ id, brief }` entries across `define`/`build`/`merge` phases, with per-brief metadata (`id`, `needs`, `generates`, `tracks`) held in each brief's YAML frontmatter. It deliberately does not model "blueprints"; that term does not appear in `schema.schema.json` or in any current `schema.yaml`.

For every `schemas/*/schema.yaml` it MUST, at minimum:

1. Parse the YAML into the `Schema` type from `specify_schema` and collect every pipeline entry across the three phases in declaration order.
2. **Pipeline id uniqueness.** Fail with `duplicate pipeline entry id '<id>'` if any `id` repeats across the combined `define`/`build`/`merge` list.
3. **Brief file resolution.** For each entry, resolve `brief` relative to the schema directory and fail with `brief not found for '<id>': <path>` if the file does not exist.
4. **Brief frontmatter parses.** Read the brief's YAML frontmatter using `specify_schema::brief`; fail with `brief '<id>' has no valid frontmatter: <path>` if it is missing or unparseable.
5. **Frontmatter id matches pipeline id.** Fail with `pipeline id '<id>' does not match brief frontmatter id '<fm.id>'` on mismatch.
6. **`needs` references declared.** For every entry in the brief's `needs` list, fail with `brief '<id>' needs undeclared '<dep>'` if `<dep>` is not one of the pipeline ids.
7. **Acyclic `needs` graph.** Build the directed graph `dep -> id` and run Kahn's algorithm; fail with `cycle in brief needs graph` if the visited count is less than the id count. The algorithm MUST match the current implementation so failure messages and ordering remain stable during migration.

#### Other modules

`links.rs`, `skill_frontmatter.rs`, `skill_references.rs`, `skill_variables.rs`, `skill_directives.rs`, `marketplace.rs`, and `plugins_doc.rs` each port one of the remaining sections from `checks.ts` (links, skill frontmatter, symlink-aware references, variable consistency, skill directives, marketplace/plugin.json alignment, `docs/plugins.md` inventory). Behaviour for each is captured in the corresponding `checks.ts` function; the port mandate is identical — preserve failure messages and semantics so `checks.ts` can be retired incrementally.

`links.rs` in particular carries load-bearing symlink behaviour (the existing `isUnderSymlink` gate governs how shared reference docs under `plugins/spec/references/` are traversed). The port MUST preserve it; see **N14 in the RFC-1 remediation checklist** for context.

### `specify check` Subcommand

The root `specify` package (`src/main.rs`) gains a single new subcommand:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing RFC-1 subcommands ...

    /// Validate the specify framework repo itself
    Check {
        /// Repository root
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
}
```

The subcommand is a thin dispatcher that runs every `specify-check` module in sequence and aggregates results into the same JSON envelope shape RFC-1 defines for `specify validate` (including the versioned `schema_version` field from M12 once that remediation lands). Exit code is non-zero iff any module reports a failure.

### Dependencies

```toml
# crates/specify-check/Cargo.toml
[dependencies]
specify-schema = { path = "../specify-schema" }
specify-error  = { path = "../specify-error" }
serde_json = "1"
jsonschema = "0.29"
```

`jsonschema` pulls in `fancy-regex`, `ahash`, `url`, and a handful of transitive crates. This is acceptable because `specify-check` is CI-only — none of that weight leaks into the root `specify` binary or the runtime domain crates. `valico` or a minimal hand-rolled validator are viable alternatives if the footprint becomes a problem later (see **N15 in the RFC-1 remediation checklist**).

### Migration Strategy

The port is a rolling migration, not a flag day:

1. **Land `specify-check` as a stub.** `specify check` wires up the CLI subcommand, depends on `specify-schema` (for schema + brief parsers) and `specify-error`, and has empty-but-compiling modules. The Makefile runs both tools; both pass trivially. This change is mechanical and self-contained.
2. **Port `schema_structure` and `schema_integrity` first.** They are the largest sections of `checks.ts` and the only ones that share code with `specify-schema` (`schema`, `brief`). Porting them first validates the parser reuse story before investing in the smaller modules.
3. **Port the remaining modules one at a time.** Each port deletes the corresponding function (or block) from `checks.ts`. Failure messages MUST match during the overlap period so CI diffs stay readable.
4. **Retire `checks.ts`.** When the script is empty, delete the file, remove the Deno dependency from the Makefile, and switch CI to `specify check` only. Document the sunset in the Makefile (see **N18 in the RFC-1 remediation checklist**).

Each step is independently mergeable and leaves CI green.

### Makefile Integration

```makefile
.PHONY: checks

checks: build
	./specify check --repo .
	@$(DENO) run --allow-read scripts/checks.ts  # keep during migration
```

`build` is defined by [RFC-1](archive/rfc-1-cli.md). The second line is removed once `checks.ts` is empty; until then both tools run side-by-side and any discrepancy between them is treated as a port regression.

## Alternatives Considered

**Keep `checks.ts` indefinitely.** Rejected because maintaining a second toolchain (Deno) for a single script adds CI friction and duplicates the schema / brief parsing logic `specify-schema` already owns. The duplication is small today, but every new schema-level invariant doubles the implementation work until the port is done.

**Stub `specify-check` in Phase 1 of RFC-1, migrate modules continuously.** This is option (ii) from the **RFC-1 remediation checklist** M6. It works, but leaves RFC-1's scope ambiguous ("done when?") and muddies the `specify` CLI release story downstream consumers track. Separating the port into its own RFC makes each milestone self-contained.

**Rewrite the linter from scratch.** Skips the fidelity constraints but throws away the invariants the current script already encodes. Those invariants capture real lessons about repo drift; preserving them verbatim is cheaper than re-deriving them, and the message-preserving migration lets CI act as a regression test for the port itself.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) — the Cargo workspace, `specify_schema::schema`, and `specify_schema::brief` this RFC builds on
- **RFC-1 Remediation checklist** — M6 (split rationale), N14 (symlink handling), N15 (jsonschema weight), N18 (Makefile sunset)
- [RFC-4: Type-Safe Skill Expression](rfc-4-dsl.md) — Option 1 is satisfied once this port is complete
