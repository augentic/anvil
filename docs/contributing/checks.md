# Consistency Checks

Framework invariants over this repo's prose and manifest surfaces are plain cargo tests: `tests/framework_quality/` (links, skills, scenarios, plugins, docs prose) and `tests/rust_quality.rs` (repo-local Rust-quality predicates). Both run inside the single CI gate, `cargo make ci` — there is no separate lint engine, verb, or `make lint` step.

```bash
cargo test --test framework_quality   # prose/manifest invariants only
cargo test --test rust_quality        # Rust-quality predicates only
cargo make ci                         # the full gate (fmt, clippy, all tests, docs, vet, deny)
```

## Editor-first vs cargo tests

Framework validation splits into two surfaces:

| Surface                                  | When it runs                                | What it covers                                                                                                                                                    |
| ---------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Editor-first (YAML/JSON LSP)**         | While you edit plain YAML or JSON           | Shape violations for files the language server can bind to a schema: `.cursor-plugin/marketplace.json` and other plain YAML/JSON artifacts that declare a schema  |
| **`tests/framework_quality/` (cargo)**   | `cargo make ci` locally and in CI           | Markdown frontmatter (`SKILL.md`, scenario docs), symlink integrity, marketplace ↔ plugin consistency, link resolution, and every other cross-file predicate       |

**Authoritative schemas** live in-tree under [`schemas/`](../../schemas) and are embedded in the `specify` binary via `schema`; the framework-quality tests validate against those embedded constants (`SKILL_JSON_SCHEMA`, `SCENARIO_JSON_SCHEMA`, `MARKETPLACE_JSON_SCHEMA`). Editors resolve the same contract by binding to the published schemas via the remote `raw.githubusercontent.com` URLs in [`.vscode/settings.json`](../../.vscode/settings.json) — there is no vendored mirror to keep in sync.

**Plain YAML/JSON wiring.** Plain YAML control files can carry a first-line schema directive when a framework or runtime schema exists:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/augentic/specify/main/schemas/plan/plan.schema.json
```

Workflow and consumer schemas (`plan`, `evidence`, …) and framework authoring schemas (`authoring/skill`, `authoring/scenario`, `authoring/marketplace`, `rules/rule`) all ship in-tree under `schemas/`. JSON manifests can use a top-level `"$schema"` property — see [`.cursor-plugin/marketplace.json`](../../.cursor-plugin/marketplace.json). (Adapters have no manifest file — adapter metadata comes from the component's `describe` export, so there is no adapter schema to bind.)

**Markdown frontmatter.** Cursor's YAML language server validates standalone `.yaml` control files reliably, but does not yet surface the same diagnostics for YAML embedded in Markdown frontmatter. The framework-quality tests extract the leading `---` block from `SKILL.md` and scenario Markdown files and validate it against the same JSON Schemas under `schemas/authoring/`.

## Enforcement surfaces (authoring vs engineering standards)

Framework and consumer validation are intentionally separate. See [Standards layer](../explanation/standards-layer.md).

| Surface                   | Command                              | Audience                           | Enforces                                                        |
| ------------------------- | ------------------------------------ | ---------------------------------- | --------------------------------------------------------------- |
| **Authoring standards**   | `cargo test --test framework_quality` | `augentic/specify` contributors    | Skill frontmatter, links, marketplace consistency, docs prose   |
| **Engineering standards** | `specify rules export`               | Consumer projects with `.specify/` | Resolved rule packs exported into the consumer's agent context  |
| **Build-time judgment**   | Target `build/review.md` briefs      | Active slice during `/spec:build`  | Model-assisted codex policy → `REVIEW.md`                       |

Rule *content* lives in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters): the shared tree at `codex/rules/universal/` (`UNI-*`) and per-adapter `prose/rules/` overlays. Rule *shape* validation (frontmatter fields, `## Rule` heading, id uniqueness, namespace ownership) also lives in that repo as a cargo test, beside the rules it validates; `rules/parse.rs` here schema-validates every rule again at each `specify rules export` as a backstop. `docs/standards/` in this repo is **authoring** house style only.

## What the framework-quality tests enforce

Each module under [`tests/framework_quality/`](../../tests/framework_quality/) owns one family. Policy (schema registries, numeric caps, allow-lists) lives as constants in the module; a violation is a test failure naming the check and the offending path.

### `links.rs`

- **Markdown link resolution** — every relative link in every `.md` file must resolve to an existing file. External links (`http://`, `mailto:`, `#` anchors) and `src/` paths are skipped; fenced code blocks and inline code spans are stripped before scanning.
- **Diagram asset embeds** — `.svg` image embeds under `docs/` must resolve.
- **Symlink integrity** — every symlink under `plugins/` must resolve to a valid target.
- **Deployable surfaces must not link into `docs/`** — markdown links under `plugins/` and adapter `prose/prompts/` + `prose/references/` trees must not escape into `docs/`.
- **Skill directive validation** — `<!-- skill: plugin:skill -->` directives must reference a real skill discovered under `plugins/`.
- **Tool-owned schema URLs** — every `schemas.specify.dev/<tool>/<name>.schema.json` URL in adapter trees must match the constant tool → schema-name registry in the module (currently `vectis` → `tokens`, `assets`, `composition`).

The judgment-prose corpus embedded by the workflow crate gets a second, stronger gate at compile time: `crates/workflow-lib/build.rs` inlines each prompt body and synthesis reference into `OUT_DIR` and link-checks it, so a dangling relative reference in that corpus **fails the build**, not just the test.

### `skills.rs`

Every `SKILL.md` under `plugins/` is validated against the embedded `schemas/authoring/skill.schema.json`, plus the predicates the schema cannot see:

- **Plugin-qualified name** — `name` is `<plugin>-<skill>` (e.g. `specify-refine`), unique across plugins, carrying the owning plugin's discovery prefix.
- **Description grammar** — the description's first word is an approved imperative verb.
- **Argument-hint grammar** — every whitespace-separated `argument-hint` token matches the `<placeholder>` / `[optional]` grammar.
- **No frontmatter restatement** — skill bodies must not restate frontmatter with `## Input`-style headings.

### `scenarios.rs`

Eval scenario files (opt-in by frontmatter, discovered under `evals/scenarios/`, `targets/*/tests/`, and promoted skill fixtures) are validated against the embedded `schemas/authoring/scenario.schema.json`, plus:

- **Stages contiguity** — `stages` must be a contiguous slice of `[plan, refine, build, merge, drop]`.
- **Body-id consistency** — a visible `Scenario ID:` body line must equal the frontmatter `id`.
- **Expected-artifact path safety** — no `..` segments or leading `/`.
- **Cross-file id uniqueness** — scenario ids are unique across the repo.
- **Catalog ↔ runs drift** — the [`evals/scenarios/README.md`](../../evals/scenarios/README.md) catalog's Status/Gate columns must agree with the committed run records under `evals/runs/`.

### `prose.rs`

- **Marketplace manifest consistency** — `.cursor-plugin/marketplace.json` lists exactly the on-disk plugins, and validates against the embedded marketplace schema.
- **Documented numeric caps** — the skill description/body caps cited in `docs/standards/skill-authoring.md` must match the embedded skill schema.
- **Canonical document presence** — `docs/reference/review-team-protocol.md` must exist (adapter `agent-teams.md` symlinks in specify-adapters resolve through it).
- **Reference-corpus indexes** — multi-file reference corpora must carry a `README.md` index.
- **No design-history citations** — prose must not cite retired RFC/design-record ids as authority.
- **No flow arrows in `text` fences** — explanation docs use real diagrams, not ASCII flow arrows in ```text fences.

## Extending the checks

Add the predicate to the owning module under `tests/framework_quality/` (or a new module wired into `main.rs::run_all`), with its policy as module constants. Then add a known-bad fixture case to the matching `*_checks_fire_on_bad_fixtures` test in `main.rs`, proving the check fires. There is no rule file, no `config:` plumbing, and no registration step — a check that compiles and runs is enforced.

Repo-local Rust predicates (test-fn naming, archaeology, `#[allow]` hygiene, the unit-test ratchet) belong in `tests/rust_quality.rs` instead; see [testing.md](../standards/testing.md).

## CLI checks

The Rust workspace at the repo root runs everything via `cargo-make` (from the repo root):

```bash
cargo make ci     # fmt-check + clippy + all tests (incl. framework_quality, rust_quality) + docs + vet + deny
cargo make check  # the pre-commit subset
```

CI is one job: [`.github/workflows/ci.yaml`](../../.github/workflows/ci.yaml) checks out this repo plus `augentic/specify-adapters` (the `SPECIFY_ADAPTERS` checkout feeds the universal rules pack embed) and runs `cargo make ci`.
