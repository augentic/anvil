# Specify Roadmap

Specify inverts the usual AI-assisted development model: an agent orchestrates skills to achieve automated code generation, with deterministic tools used only where precision is required. This roadmap is structured as RFCs — each one addresses a specific structural weakness in the current framework.

---

## [RFC-1: `specify` CLI](archive/rfc-1-cli.md)

**Status:** Shipped (Phase 1, 2026-04). See [DECISIONS.md](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) in `augentic/specify-cli` for the architectural calls made during the build.

**Problem:** Every precision-critical operation — validation, task parsing, artifact structure checking — is performed by the LLM interpreting prose rules. This creates unreliable results for operations that are fundamentally structured decision trees.

**Solution:** A Rust CLI binary (`specify`) that owns every deterministic operation. Skills invoke it via shell commands and get structured JSON or exit codes back. The agent keeps judgment; the CLI keeps correctness. Replaces `merge-specs.py`, the 40+ lines of prose validation in the build skill, and scattered mkdir/copy/write logic across init, define, and status.

The CLI is the foundation everything else builds on. Migration commands, multi-repo coordination, and skill validation all require a binary that understands `.specify/` structure, spec format, and schema rules. Building the CLI first means every subsequent RFC extends an existing tool rather than creating a new one.

See also: [RFC-1a: Deferred Validation](archive/rfc-1a-validation.md) — the three-way Pass/Fail/Deferred classification that lets the CLI handle structural checks while the agent evaluates semantic ones.

## [RFC-2: Execution](archive/rfc-2-execution.md)

**Status:** Implemented (Layers 1–3, 2026-04). Plan format, `specify initiative` CLI, `/spec:execute` driver skill, and `/spec:plan` authoring skill are all live. See [DECISIONS.md](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) in `augentic/specify-cli` for the architectural calls made during the build.

**Problem:** Complex initiatives — multi-feature greenfield builds, legacy migrations, platform modernisations — lack a coordination artifact. The agent rediscovers scope, ordering, and dependencies on every iteration. There's no persistent plan that tracks what's done, what's next, and what's blocked.

**Solution:** A Plan (`plan.yaml`) that drives the same define-build-merge loop Specify already uses, change by change, with dependency tracking and progressive baseline accumulation. For legacy migration, a change adds `sources` and define's extract sub-skill analyses them. For greenfield work, define starts from the description. The loop is the same either way. Extracted and greenfield changes coexist in a single plan. Layer 2 adds the `/spec:execute` driver skill (and Layer 3's `/spec:plan` entry-writer skill) that automate the loop.

## [RFC-3: Multi-Repo Planning](rfc-3a-plan.md)

**Problem:** RFC-2 assumes you already know the changes. For legacy modernisation, greenfield builds across multiple repos, and platform-wide initiatives, the agent has to *derive* the changes from inputs (legacy code, documentation) and coordinate them across a set of repos whose scope isn't declared anywhere. RFC-2's `/spec:plan` handles single-repo plan authoring; the multi-repo case has no equivalent.

**Solution:** Extend `/spec:plan` to be registry-aware, and add a fixed *sync peers* phase that runs automatically when `registry.yaml` is present, cloning peer repos into `.specify/workspace/` and inventorying their specs. Inputs are dispatched by `kind`: legacy code to `/spec:extract`, documentation to a new `/spec:analyze` skill; the vocabulary is a closed enum. The same `/spec:plan <name>` command scales unchanged from a single repo to 100+ repos, with no planning configuration file. Layer 1 introduces `registry.yaml` and the discovery-dispatch contract; Layer 2 adds the sync-peers phase for multi-repo runs; Layer 3 adds federation at execution time (cross-repo spec references, contract reconciliation) on top of the same workspace.

## [RFC-4: Type-Safe Skill Expression](rfc-4-dsl.md)

**Problem:** Skills have two distinct layers — structural metadata (dependencies, tools, arguments, references) and behavioral instructions (prose) — but both live in untyped markdown. Breaking a reference, misspelling a tool name, or introducing a variable inconsistency produces no feedback until runtime.

**Solution:** Extend the CLI's validation surface to cover skill authoring: frontmatter schema enforcement, reference resolution, variable consistency, and cross-skill directive validation. As the skill count grows, graduate to structured YAML manifests or a Rust DSL that separates the typed skeleton from the prose body, giving progressively stronger "compile-time" feedback. Depends on the CLI from RFC-1 for the validation infrastructure.

Note: `checks.ts` already implements the core of Option 1 (CLI-integrated skill validation) — frontmatter schema enforcement, reference resolution, variable consistency, skill directive validation, marketplace consistency, and docs inventory checks. The primary gap is porting these checks from Deno TypeScript into the `specify-check` crate, which is the subject of [RFC-5](rfc-5-framework-lint.md). Stronger typing (Options 2–3) is deferred until the skill count or composability needs justify the investment.

## [RFC-5: Framework Linter](rfc-5-framework-lint.md)

**Problem:** The repo's framework-level invariants — schema ↔ JSON-Schema conformance, brief frontmatter integrity, marketplace.json alignment, SKILL.md reference resolution, cross-skill directive validity, docs/plugins inventory — are enforced by `scripts/checks.ts`, a ~650-line Deno script. It works, but it keeps a second toolchain (Deno) alive purely for CI and duplicates the schema / brief parsing logic `specify-schema` already owns.

**Solution:** Port `checks.ts` into a Rust `specify-check` crate exposed as `specify check`, module-for-module, preserving failure messages so CI diffs stay readable while both tools run side-by-side. Depends on the CLI from RFC-1 but is deliberately decoupled from its Phase 1 scope: the runtime CLI ships without waiting on the framework linter, and the port proceeds as a focused background migration. Once parity is reached, `checks.ts` and the Deno dependency are removed. Satisfies [RFC-4](rfc-4-dsl.md)'s Option 1 on the way through.

## [RFC-6: Vectis Bootstrap CLI](rfc-6-vectis-bootstrap.md)

**Problem:** Bootstrapping a greenfield Crux cross-platform project requires the agent to interpret ~3,000 lines of prose instructions across three writer skills (core-writer, ios-writer, android-writer), write ~40 files one at a time, and iterate on compilation errors. The output is fully deterministic — given an app name, capabilities, and target platforms — yet the agent spends 10-20 minutes and significant tokens on work that requires no judgment.

**Solution:** A Rust CLI binary (`vectis`) with four subcommands: `vectis init` scaffolds a minimum-viable Crux project (core always, iOS and Android shells optionally) from embedded templates with correct version pins; `vectis add-shell` adds a platform shell to an existing core-only project by parsing `app.rs` for the app name and capabilities; `vectis verify` checks all assemblies compile; and `vectis update-versions` manages coherent dependency pins across the Crux, Android, and iOS ecosystems. All commands perform prerequisite detection first and stop with a clear report if required toolchain components are missing. Writer skills detect greenfield projects and invoke the CLI before switching to Update Mode for feature-specific implementation. Independent of the `specify` CLI from [RFC-1](archive/rfc-1-cli.md) — different binary, different purpose, different lifecycle.
