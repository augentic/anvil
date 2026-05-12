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
| `checkBodyAndSectionLineCounts` | SKILL.md body length over the hard cap of **200 lines**, and per-H2 section over **45 lines** (non-blank, non-comment). | [scripts/checks/skill_body.ts](../../scripts/checks/skill_body.ts) | Per-file `bodyLineCount` / `sectionLineCount` baselines in `scripts/standards-allowlist.toml` grandfather oversized files. |
| `checkDescriptionHasUseWhen` | `description` field missing the `Use when …` trigger clause. | [scripts/checks/skill_frontmatter.ts](../../scripts/checks/skill_frontmatter.ts) | No per-file baseline. |
| `checkDescriptionLength` | `description` field over the hard cap of **512 chars**. | [scripts/checks/skill_frontmatter.ts](../../scripts/checks/skill_frontmatter.ts) | No per-file baseline. |
| `checkDescriptionStartsWithVerb` | `description` field that does not start with an imperative verb from the curated allow-list in `scripts/checks/skill_frontmatter.ts` (`IMPERATIVE_VERBS`). | [scripts/checks/skill_frontmatter.ts](../../scripts/checks/skill_frontmatter.ts) | Allow-list is extended in-place when a new verb is genuinely imperative. |
| `checkNoEnvelopeExamples` | Fenced ` ```json ` / ` ```jsonc ` blocks whose body looks like a full CLI envelope — primarily the `"envelope-version"` key (the marker of the flat envelope contract); a legacy `"ok"` + `"data"` / `"error"` pair is still matched so old embeddings cannot sneak back in. Link to `plugins/references/cli-output-shapes.md` instead. | [scripts/checks/skill_body.ts](../../scripts/checks/skill_body.ts) | Per-file baseline in `scripts/standards-allowlist.toml`. |
| `checkOperationalVocabulary` | Active prose using retired slice paths, top-level CLI commands, or pre-cutover umbrella nouns outside archived/historical material. | [scripts/checks/prose.ts](../../scripts/checks/prose.ts) | Per-file baseline. |
| `checkSkillNumericCaps` | The 512 / 200 / 45 caps drifting out of sync across scripts, schema, rules, and docs. | [scripts/checks/prose.ts](../../scripts/checks/prose.ts) | No per-file baseline — caps are kept synchronized. |

## Adjacent checks

`checks.ts` also runs link integrity, capability schema, declared-tool, plugin shape, scenario, codex agent, and docs-quality checks (`links.ts`, `capability.ts`, `tools.ts`, `plugins.ts`, `scenarios.ts`, `codex.ts`, `docs_quality.ts`). Those are not predicates over skill bodies, but they share the same `make checks` orchestration and the same allowlist file; failures surface alongside the predicates above.
