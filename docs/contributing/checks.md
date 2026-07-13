# Consistency Checks

Framework invariants over this repo's prose and manifest surfaces are plain cargo tests at `tests/framework/` (links, skills, plugins, architecture boundaries). They run inside the single CI gate, `cargo make ci` — there is no separate lint engine, verb, or `make lint` step.

```bash
cargo test --test framework   # prose/manifest invariants only
cargo make ci                 # the full gate (fmt, clippy, all tests, docs, vet, deny)
```

The suite is deliberately small and risk-based: it enforces only contracts whose breakage ships a broken artifact (plugin packaging, skill discovery, symlinks), violates the operating model (orchestration prose in skill wrappers, adapter crates in the engine), or rots silently on every PR (docs links). House style stays in `docs/standards/` as guidance, not as CI predicates.

## Who owns which invariant

| Invariant | Owner | When it runs |
| --------------------------------------------- | ------------------------------------------------------ | ----------------------------- |
| Plugin/skill/marketplace packaging, boundaries | `tests/framework/` (cargo) | Every `cargo make ci` |
| Embedded prompt-corpus links | `crates/prose` via `crates/slice/build.rs` + `crates/change/build.rs` — a dangling reference **fails the build** | Every compile |
| Published docs book links | mdbook-linkcheck2 in [`.github/workflows/docs.yaml`](../../.github/workflows/docs.yaml) | Push to `main` for `docs/**` only — **not** a PR gate |
| `docs/` relative links and SVG embeds on PRs | `tests/framework/links.rs` | Every `cargo make ci` |
| Authoring schema shape (`skill`, `marketplace`) | `schemas/authoring/*.schema.json`, embedded via the `schema` crate; instance validation by `tests/framework/` | Every `cargo make ci` |

## Editor-first vs cargo tests

Framework validation splits into two surfaces:

| Surface                          | When it runs                      | What it covers                                                                                                                                                   |
| -------------------------------- | --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Editor-first (YAML/JSON LSP)** | While you edit plain YAML or JSON | Shape violations for files the language server can bind to a schema: `.cursor-plugin/marketplace.json` and other plain YAML/JSON artifacts that declare a schema |
| **`tests/framework/` (cargo)**   | `cargo make ci` locally and in CI | Markdown frontmatter (`SKILL.md`), symlink integrity, marketplace ↔ plugin consistency, link resolution, and the architecture boundary                           |

**Authoritative schemas** live in-tree under [`schemas/`](../../schemas) and are embedded in the `specify` binary via `schema`; the framework-quality tests validate against those embedded constants (`SKILL_JSON_SCHEMA`, `MARKETPLACE_JSON_SCHEMA`).

**Plain YAML/JSON wiring.** Plain YAML control files can carry a first-line schema directive when a framework or runtime schema exists:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/augentic/specify/main/schemas/plan/plan.schema.json
```

Workflow and consumer schemas (`plan`, `evidence`, …) and framework authoring schemas (`authoring/skill`, `authoring/marketplace`) all ship in-tree under `schemas/`. JSON manifests can use a top-level `"$schema"` property — see [`.cursor-plugin/marketplace.json`](../../.cursor-plugin/marketplace.json). (Adapters have no manifest file — adapter metadata comes from the component's `metadata` export, so there is no adapter schema to bind.)

**Markdown frontmatter.** Cursor's YAML language server validates standalone `.yaml` control files reliably, but does not yet surface the same diagnostics for YAML embedded in Markdown frontmatter. The framework-quality tests extract the leading `---` block from `SKILL.md` files and validate it against the same JSON Schemas under `schemas/authoring/`.

## Enforcement surfaces (authoring vs engineering standards)

Framework and consumer validation are intentionally separate. See [Standards layer](../explanation/standards-layer.md).

| Surface                   | Command                         | Audience                           | Enforces                                                       |
| ------------------------- | ------------------------------- | ---------------------------------- | -------------------------------------------------------------- |
| **Authoring standards**   | `cargo test --test framework`   | `augentic/specify` contributors    | Skill frontmatter, links, marketplace consistency              |
| **Engineering standards** | Adapter-embedded rule packs     | Consumer projects with `.specify/` | Rules shipped inside each target adapter, applied by its build review prompts |
| **Build-time judgment**   | Target `build/review.md` briefs | Active slice during `/spec:build`  | Model-assisted codex policy → `REVIEW.md`                      |

Rule *content* lives in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters): the shared tree at `codex/rules/universal/` (`UNI-*`) and per-adapter `prose/rules/` overlays, embedded in each adapter's published component. Rule *shape* validation (frontmatter fields, `## Rule` heading, id uniqueness, namespace ownership) also lives in that repo as a cargo test, beside the rules it validates. `docs/standards/` in this repo is **authoring** house style only.

## What the framework-quality tests enforce

Each module under [`tests/framework/`](../../tests/framework/) owns one family. Policy lives as constants in the module; a violation is a test failure naming the check and the offending path.

### `boundaries.rs`

- **Adapter dependency direction** — no engine Cargo manifest (workspace root, `crates/`, `harness/`) may depend on a concrete adapter crate or reach into the `specify-adapters` repository via a `path`/`git` source. The engine talks to adapters only through the WASM component seam.

### `links.rs`

- **Markdown link resolution** — every relative link under `plugins/`, `docs/`, and `.cursor/` must resolve to an existing file. External links (`http://`, `mailto:`, `#` anchors) are skipped; fenced code blocks and inline code spans are stripped before scanning. This is the only PR-time gate for `docs/` links — mdBook linkcheck runs post-merge only.
- **Diagram asset embeds** — `.svg` image embeds under `docs/` must resolve.
- **Symlink integrity** — every symlink under `plugins/` must resolve to a valid target.
- **Deployable surfaces must not link into `docs/`** — markdown links under `plugins/` must not escape into the non-shipped `docs/` tree.
- **Permanent surfaces must not link into `rfcs/`** — `rfcs/` is disposable working design; docs, plugins, and the embedded prompt corpus must not cite it.
- **Skill directive validation** — `<!-- skill: plugin:skill -->` directives must reference a real skill discovered under `plugins/`.

The judgment-prose corpus embedded by the workflow crates is out of the framework link walk: `crates/slice/build.rs` and `crates/change/build.rs` inline each prompt body into `OUT_DIR` and link-check it, so a dangling relative reference in that corpus **fails the build**.

### `skills.rs`

Every `SKILL.md` under `plugins/` is validated against the embedded `schemas/authoring/skill.schema.json` (name pattern, description shape, `argument-hint` grammar), plus the predicates the schema cannot see:

- **Plugin-qualified name** — `name` is `<plugin>-<skill>` (e.g. `specify-refine`), unique across plugins, carrying the owning plugin's discovery prefix.
- **No frontmatter restatement** — skill bodies must not restate frontmatter with `## Input`-style headings.
- **No orchestration headings** — `spec` skill bodies must not carry headings naming engine-owned behavior (synthesis, reconciliation, validation, lifecycle, …); skills are invoke-and-relay wrappers.

### `prose.rs`

- **Marketplace manifest consistency** — `.cursor-plugin/marketplace.json` lists exactly the on-disk plugins, and validates against the embedded marketplace schema.
- **Canonical document presence** — `docs/reference/review-team-protocol.md` must exist (adapter `agent-teams.md` symlinks in specify-adapters resolve through it).

## Extending the checks

Add the predicate to the owning module under `tests/framework/` (or a new module wired into `main.rs::run_all`), with its policy as module constants. Then add a known-bad fixture case to the matching `bad_fixtures` test in `main.rs`, proving the check fires. There is no rule file, no `config:` plumbing, and no registration step — a check that compiles and runs is enforced.

Before adding a check, ask whether its failure breaks a shipped artifact or an architecture contract. House-style preferences (naming caps, verb lists, diagram formats) belong in `docs/standards/` as guidance, not here.

## CLI checks

The Rust workspace at the repo root runs everything via `cargo-make` (from the repo root):

```bash
cargo make ci     # fmt-check + clippy + all tests (incl. framework) + docs + vet + deny
cargo make check  # the pre-commit subset
```

Per-push CI ([`.github/workflows/ci.yaml`](../../.github/workflows/ci.yaml)) needs no sibling checkout: the shared org workflow runs the workspace gates (its test leg covers the default members: `crates/*`, `harness/fixtures`, and `tests/framework`), and a slim job checks the guest crates compile for `wasm32-wasip2`. WASM boundary execution lives in the weekly/path-filtered [`.github/workflows/wasm.yaml`](../../.github/workflows/wasm.yaml).
