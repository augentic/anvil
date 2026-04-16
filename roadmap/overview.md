# Specify Roadmap

Specify inverts the usual AI-assisted development model: an agent orchestrates skills to achieve automated code generation, with deterministic tools used only where precision is required. This roadmap is structured as RFCs — each one addresses a specific structural weakness in the current framework.

---

## [RFC-1: `specify` CLI](rfc-1-cli.md)

**Problem:** Every precision-critical operation — validation, task parsing, artifact structure checking — is performed by the LLM interpreting prose rules. This creates unreliable results for operations that are fundamentally structured decision trees.

**Solution:** A Rust CLI binary (`specify`) that owns every deterministic operation. Skills invoke it via shell commands and get structured JSON or exit codes back. The agent keeps judgment; the CLI keeps correctness. Replaces `merge-specs.py`, the 40+ lines of prose validation in the build skill, and scattered mkdir/copy/write logic across init, define, and status.

The CLI is the foundation everything else builds on. Migration commands, multi-repo coordination, and skill validation all require a binary that understands `.specify/` structure, spec format, and schema rules. Building the CLI first means every subsequent RFC extends an existing tool rather than creating a new one.

See also: [RFC-1a: Deferred Validation](rfc-1a-validation.md) — the three-way Pass/Fail/Deferred classification that lets the CLI handle structural checks while the agent evaluates semantic ones.

## [RFC-2: Iterative Legacy Migration](rfc-2-migration.md)

**Problem:** Legacy migration typically fails because the new system diverges from the old system's actual behaviour — the behaviour nobody wrote down. Big-bang rewrites take months before delivering value, and when they stall, everything is lost.

**Solution:** Reuse the same define-build-merge loop that powers greenfield development, feature by feature, driven by a migration manifest. Each slice of the legacy system is extracted, defined, built, and merged in a self-contained iteration. The baseline grows with every merge; progress is incremental and reversible. Existing skills (`/spec:extract`, `wiretapper`, `replay-writer`) provide the extraction and verification infrastructure.

The migration design is ready — the loop, manifest format, and slice strategy are fully specified. Implementation depends on the CLI from RFC-1: the deterministic operations (`specify migrate init`, `specify migrate next`, `specify migrate status`) are natural extensions of the CLI's subcommand surface. The skill-level orchestration (extract → define → build → merge) already works today; what's missing is the manifest-driven automation layer.

## [RFC-3: Multi-Repo Coordination](rfc-3-multi-repo.md)

**Problem:** The `.specify/` directory is project-local. There is no concept of a spec reference that spans repositories, and conflict detection only works within a single workspace. A feature like "add OAuth login" that touches backend, frontend, and shared-types repos has no coordination point.

**Solution:** Extend `config.yaml` with a federation model — peer repositories declared in config, cross-repo spec references resolved by the CLI, and coordinated validation that catches contract mismatches across repo boundaries.

## [RFC-4: Type-Safe Skill Expression](rfc-4-dsl.md)

**Problem:** Skills have two distinct layers — structural metadata (dependencies, tools, arguments, references) and behavioral instructions (prose) — but both live in untyped markdown. Breaking a reference, misspelling a tool name, or introducing a variable inconsistency produces no feedback until runtime.

**Solution:** Extend the CLI's validation surface to cover skill authoring: frontmatter schema enforcement, reference resolution, variable consistency, and cross-skill directive validation. As the skill count grows, graduate to structured YAML manifests or a Rust DSL that separates the typed skeleton from the prose body, giving progressively stronger "compile-time" feedback. Depends on the CLI from RFC-1 for the validation infrastructure.

Note: `checks.ts` already implements the core of Option 1 (CLI-integrated skill validation) — frontmatter schema enforcement, reference resolution, variable consistency, skill directive validation, marketplace consistency, and docs inventory checks. The primary gap is porting these checks from Deno TypeScript into the `specify-check` crate, which happens naturally as part of RFC-1's `specify check` subcommand. Stronger typing (Options 2–3) is deferred until the skill count or composability needs justify the investment.

## [RFC-5: Vectis Bootstrap CLI](rfc-5-vectis-bootstrap.md)

**Problem:** Bootstrapping a greenfield Crux cross-platform project requires the agent to interpret ~3,000 lines of prose instructions across three writer skills (core-writer, ios-writer, android-writer), write ~40 files one at a time, and iterate on compilation errors. The output is fully deterministic — given an app name, capabilities, and target platforms — yet the agent spends 10-20 minutes and significant tokens on work that requires no judgment.

**Solution:** A Rust CLI binary (`vectis`) with four subcommands: `vectis init` scaffolds a minimum-viable Crux project (core always, iOS and Android shells optionally) from embedded templates with correct version pins; `vectis add-shell` adds a platform shell to an existing core-only project by parsing `app.rs` for the app name and capabilities; `vectis verify` checks all assemblies compile; and `vectis update-versions` manages coherent dependency pins across the Crux, Android, and iOS ecosystems. All commands perform prerequisite detection first and stop with a clear report if required toolchain components are missing. Writer skills detect greenfield projects and invoke the CLI before switching to Update Mode for feature-specific implementation. Independent of RFC-1 — different binary, different purpose.
