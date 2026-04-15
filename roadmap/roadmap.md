# Specify Roadmap

Specify inverts the usual AI-assisted development model: an agent orchestrates skills to achieve automated code generation, with deterministic tools used only where precision is required. This roadmap is structured as horizons — each one addresses a specific structural weakness in the current framework.

---

## [Horizon 1: `specify` CLI](cli.md)

**Problem:** Every precision-critical operation — validation, task parsing, artifact structure checking — is performed by the LLM interpreting prose rules. This creates unreliable results for operations that are fundamentally structured decision trees.

**Solution:** A Rust CLI binary (`specify`) that owns every deterministic operation. Skills invoke it via shell commands and get structured JSON or exit codes back. The agent keeps judgment; the CLI keeps correctness. Replaces `merge-specs.py`, the 40+ lines of prose validation in the build skill, and scattered mkdir/copy/write logic across init, define, and status.

See also: [Deferred validation](validation.md) — the three-way Pass/Fail/Deferred classification that lets the CLI handle structural checks while the agent evaluates semantic ones.

## [Horizon 2: Multi-Repo Coordination](multi-repo.md)

**Problem:** The `.specify/` directory is project-local. There is no concept of a spec reference that spans repositories, and conflict detection only works within a single workspace. A feature like "add OAuth login" that touches backend, frontend, and shared-types repos has no coordination point.

**Solution:** Extend `config.yaml` with a federation model — peer repositories declared in config, cross-repo spec references resolved by the CLI, and coordinated validation that catches contract mismatches across repo boundaries.

## [Horizon 3: Iterative Legacy Migration](migration.md)

**Problem:** Legacy migration typically fails because the new system diverges from the old system's actual behaviour — the behaviour nobody wrote down. Big-bang rewrites take months before delivering value, and when they stall, everything is lost.

**Solution:** Reuse the same define-build-merge loop that powers greenfield development, feature by feature, driven by a migration manifest. Each slice of the legacy system is extracted, defined, built, and merged in a self-contained iteration. The baseline grows with every merge; progress is incremental and reversible. Existing skills (`code-analyzer`, `wiretapper`, `replay-writer`) provide the extraction and verification infrastructure.

## [Appendix: Type-Safe Skill Expression](dsl.md)

**Problem:** Skills are expressed in natural language with no structural validation beyond YAML frontmatter conventions. Breaking a link or referencing a non-existent artifact produces no feedback until runtime.

**Solution:** Explores options for giving skills "compile-time" feedback — from typed DSLs (BAML, Rust codegen) to lighter-weight schema enforcement. Currently deferred; the CLI from Horizon 1 addresses the most acute validation gaps first.
