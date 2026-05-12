# Predicates

Mechanical-enforcement predicates run by `make checks` (via [scripts/checks.ts](../../scripts/checks.ts), a thin orchestrator over the per-concern modules under [scripts/checks/](../../scripts/checks/)). Each row names a predicate, what it catches, where the implementation lives, and how the allowlist baseline works for it.

This table mirrors in spirit the predicates table maintained in the sibling `specify-cli` repository, but the engines differ: this repo's checks run on Markdown and YAML through Deno (`checks.ts`), while the CLI repo enforces source-code predicates over Rust with `syn` + `regex` through an `xtask` runner. Skills and tests grep both surfaces.

## Allowlist policy

Per-predicate per-file baselines for the skill-body discipline live in [scripts/standards-allowlist.toml](../../scripts/standards-allowlist.toml). A live count strictly greater than its baseline fails CI. New files start clean (missing entries default to 0).

**Ratchet** — any PR that touches a skill is expected to reduce its baselines where it can. A predicate baseline is grandfathering, not a license; raising a number requires a justification in the PR description.

## Predicate table

| Predicate | What it catches | Implementation | Allowlist |
|---|---|---|---|
| `checkArgumentHintGrammar` | `argument-hint:` tokens that don't match the canonical grammar (`<name>`, `[name]`, trailing `...`, `<a\|b>`, `[a\|b]`, `--flag`). Bare prose, mixed punctuation, trailing `?`, and short flags rejected. | [scripts/checks/skill_frontmatter.ts](../../scripts/checks/skill_frontmatter.ts) | No per-file baseline — every SKILL.md must pass. |
| `checkBodyLineCount` | SKILL.md body length over the hard cap of **250 lines**. | [scripts/checks/skill_body.ts](../../scripts/checks/skill_body.ts) | Per-file `bodyLineCount` baselines in `scripts/standards-allowlist.toml` grandfather oversized files. |
| `checkDescriptionHasUseWhen` | `description` field missing the `Use when …` trigger clause. | [scripts/checks/skill_frontmatter.ts](../../scripts/checks/skill_frontmatter.ts) | No per-file baseline. |
| `checkDescriptionLength` | `description` field over the hard cap of **512 chars**. | [scripts/checks/skill_frontmatter.ts](../../scripts/checks/skill_frontmatter.ts) | No per-file baseline. |
| `checkDescriptionStartsWithVerb` | `description` field that does not start with an imperative verb from the curated allow-list in `scripts/checks/skill_frontmatter.ts` (`IMPERATIVE_VERBS`). | [scripts/checks/skill_frontmatter.ts](../../scripts/checks/skill_frontmatter.ts) | Allow-list is extended in-place when a new verb is genuinely imperative. |
| `checkNoEnvelopeExamples` | Fenced ` ```json ` / ` ```jsonc ` blocks whose body looks like a full CLI envelope — primarily the `"envelope-version"` key (the marker of the flat envelope contract); a legacy `"ok"` + `"data"` / `"error"` pair is still matched so old embeddings cannot sneak back in. Link to `plugins/references/cli-output-shapes.md` instead. | [scripts/checks/skill_body.ts](../../scripts/checks/skill_body.ts) | Per-file baseline in `scripts/standards-allowlist.toml`. |
| `checkNoFrontmatterRestatement` | Frontmatter `description` value re-appearing under the first H2. | [scripts/checks/skill_discipline.ts](../../scripts/checks/skill_discipline.ts) | Per-file baseline. |
| `checkNoPhaseOutcomeContractRestatement` | Restated phase-outcome contract paragraph. Use the one-line link form (`> See [Phase outcome contract](../../references/phase-outcome-contract.md).`). | [scripts/checks/skill_discipline.ts](../../scripts/checks/skill_discipline.ts) | Per-file baseline. |
| `checkNoRfcCitationsInSkillBody` | `RFC[- ]?\d+` in skill body. Fenced code excluded; `rfcs/` archive links excluded. Move citations to a trailing `## References` block or to `docs/explanation/decision-log.md`. | [scripts/checks/skill_discipline.ts](../../scripts/checks/skill_discipline.ts) | Per-file baseline. |
| `checkOneGuardrailsBlockPerSkill` | Count of `## Guardrails` and `## Mode-specific guardrails` headings — exactly one per SKILL.md. | [scripts/checks/skill_discipline.ts](../../scripts/checks/skill_discipline.ts) | No per-file baseline — every SKILL.md must comply. |
| `checkOperationalVocabulary` | Active prose using retired slice paths, top-level CLI commands, or pre-cutover umbrella nouns outside archived/historical material. | [scripts/checks/prose.ts](../../scripts/checks/prose.ts) | Per-file baseline. |
| `checkSectionLineCount` | Per-H2 section over the hard cap of **60 lines** (non-blank, non-comment). | [scripts/checks/skill_body.ts](../../scripts/checks/skill_body.ts) | Per-file `sectionLineCount` baselines in `scripts/standards-allowlist.toml`. |
| `checkSkillNumericCaps` | The 512 / 250 / 60 caps drifting out of sync across scripts, schema, rules, and docs. | [scripts/checks/skill_body.ts](../../scripts/checks/skill_body.ts) | No per-file baseline — caps are kept synchronized. |

## Sibling Rust predicates (CLI repo)

The following predicates are enforced over Rust source in the sibling `specify-cli` repo's `xtask standards-check`, not by `scripts/checks.ts` here. They have no skill/markdown analogue but are listed for inventory parity with [`specify-cli/docs/standards/predicates.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/predicates.md).

| Predicate | What it catches | Implementation | Allowlist |
|---|---|---|---|
| `crate-root-prose` | A `lib.rs` or `main.rs` whose leading `//!` doc paragraph exceeds 30 non-blank lines. Architectural prose belongs in `docs/standards/` or an in-repo RFC. | `specify-cli/xtask/src/standards.rs` | Per-file baseline in `specify-cli/scripts/standards-allowlist.toml`. |
| `display-serde-mirror` | `impl Display for T` where `T` derives `Serialize` and the body is `match self { Self::Variant => "literal" }`. | `specify-cli/xtask/src/standards.rs` | Per-file baseline; defaults to 0 — fix by extracting a `const fn discriminant(&self) -> &'static str` (or `as_str`) and delegating from `Display::fmt` via `f.write_str(self.discriminant())`. |
| `unit-test-serde-roundtrip` | A `#[test]` with a matching `serde_*::to_string` + `serde_*::from_str` pair. Soft predicate; allowlist when a custom Visitor or similar warrants the in-crate test. | `specify-cli/xtask/src/standards.rs` | Per-file baseline. |

## Adjacent checks

`checks.ts` also runs link integrity, capability schema, declared-tool, plugin shape, scenario, codex agent, and docs-quality checks (`links.ts`, `capability.ts`, `tools.ts`, `plugins.ts`, `scenarios.ts`, `codex.ts`, `docs_quality.ts`). Those are not predicates over skill bodies, but they share the same `make checks` orchestration and the same allowlist file; failures surface alongside the predicates above.
