# Specify Roadmap

Specify inverts the usual AI-assisted development model: an agent orchestrates skills to achieve automated code generation, with deterministic tools used only where precision is required. This roadmap is structured as RFCs — each one addresses a specific structural weakness in the current framework.

---

## [RFC-1: `specify` CLI](rfc-1-cli.md)

**Status:** Shipped (Phase 1, 2026-04). See the [implementation plan](rfc-1-plan.md) for the twelve-Change delivery and [DECISIONS.md](../DECISIONS.md) for the architectural calls made during the build.

**Problem:** Every precision-critical operation — validation, task parsing, artifact structure checking — is performed by the LLM interpreting prose rules. This creates unreliable results for operations that are fundamentally structured decision trees.

**Solution:** A Rust CLI binary (`specify`) that owns every deterministic operation. Skills invoke it via shell commands and get structured JSON or exit codes back. The agent keeps judgment; the CLI keeps correctness. Replaces `merge-specs.py`, the 40+ lines of prose validation in the build skill, and scattered mkdir/copy/write logic across init, define, and status.

The CLI is the foundation everything else builds on. Migration commands, multi-repo coordination, and skill validation all require a binary that understands `.specify/` structure, spec format, and schema rules. Building the CLI first means every subsequent RFC extends an existing tool rather than creating a new one.

See also: [RFC-1a: Deferred Validation](rfc-1a-validation.md) — the three-way Pass/Fail/Deferred classification that lets the CLI handle structural checks while the agent evaluates semantic ones.

## [RFC-2: Feature Manifests](rfc-2-manifests.md)

**Problem:** Complex initiatives — multi-feature greenfield builds, legacy migrations, platform modernisations — lack a coordination artifact. The agent rediscovers scope, ordering, and dependencies on every iteration. There's no persistent plan that tracks what's done, what's next, and what's blocked.

**Solution:** A feature manifest (`manifest.yaml`) that drives the same define-build-merge loop Specify already uses, feature by feature, with dependency tracking and progressive baseline accumulation. For legacy migration, the manifest adds `source-paths` and the loop starts with `/spec:extract`. For greenfield work, it starts at `/spec:define`. The loop is the same either way. Extracted and greenfield features coexist in a single manifest.

The manifest design is ready — the loop, format, and dependency strategy are fully specified. Implementation depends on the CLI from RFC-1: the deterministic operations (`specify manifest init`, `specify manifest next`, `specify manifest status`) are natural extensions of the CLI's subcommand surface. The skill-level orchestration already works today; what's missing is the manifest-driven automation layer.

## [RFC-3: Multi-Repo Coordination](rfc-3-multi-repo.md)

**Problem:** The `.specify/` directory is project-local. There is no concept of a spec reference that spans repositories, and conflict detection only works within a single workspace. A feature like "add OAuth login" that touches backend, frontend, and shared-types repos has no resolution layer for cross-repo spec references.

**Solution:** A federation model — peer repositories declared in config, cross-repo spec references resolved by the CLI, and coordinated validation that catches contract mismatches across repo boundaries. Feature manifests (RFC-2) provide the coordination layer for multi-repo initiatives; federation provides the resolution and validation layer that makes cross-repo references work.

## [RFC-4: Type-Safe Skill Expression](rfc-4-dsl.md)

**Problem:** Skills have two distinct layers — structural metadata (dependencies, tools, arguments, references) and behavioral instructions (prose) — but both live in untyped markdown. Breaking a reference, misspelling a tool name, or introducing a variable inconsistency produces no feedback until runtime.

**Solution:** Extend the CLI's validation surface to cover skill authoring: frontmatter schema enforcement, reference resolution, variable consistency, and cross-skill directive validation. As the skill count grows, graduate to structured YAML manifests or a Rust DSL that separates the typed skeleton from the prose body, giving progressively stronger "compile-time" feedback. Depends on the CLI from RFC-1 for the validation infrastructure.

Note: `checks.ts` already implements the core of Option 1 (CLI-integrated skill validation) — frontmatter schema enforcement, reference resolution, variable consistency, skill directive validation, marketplace consistency, and docs inventory checks. The primary gap is porting these checks from Deno TypeScript into the `specify-check` crate, which is the subject of [RFC-5](rfc-5-framework-lint.md). Stronger typing (Options 2–3) is deferred until the skill count or composability needs justify the investment.

## [RFC-5: Framework Linter](rfc-5-framework-lint.md)

**Problem:** The repo's framework-level invariants — schema ↔ JSON-Schema conformance, brief frontmatter integrity, marketplace.json alignment, SKILL.md reference resolution, cross-skill directive validity, docs/plugins inventory — are enforced by `scripts/checks.ts`, a ~650-line Deno script. It works, but it keeps a second toolchain (Deno) alive purely for CI and duplicates the schema / brief parsing logic `specify-schema` already owns.

**Solution:** Port `checks.ts` into a Rust `specify-check` crate exposed as `specify check`, module-for-module, preserving failure messages so CI diffs stay readable while both tools run side-by-side. Depends on the CLI from RFC-1 but is deliberately decoupled from its Phase 1 scope: the runtime CLI ships without waiting on the framework linter, and the port proceeds as a focused background migration. Once parity is reached, `checks.ts` and the Deno dependency are removed. Satisfies [RFC-4](rfc-4-dsl.md)'s Option 1 on the way through.
