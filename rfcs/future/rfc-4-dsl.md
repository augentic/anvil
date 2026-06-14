# RFC-4: Type-Safe Skill Expression

> Status: Option 1 shipped · Options 2–3 parked · Surface: [`specify lint framework`](https://github.com/augentic/specify-cli)

## Abstract

Skill authoring has two layers with different validation needs: **structural metadata** (what a skill depends on, which tools it uses, what arguments it takes, which phases it runs, which artifacts it references) that can be checked mechanically, and **behavioral instructions** (how the agent should think and act) that are inherently natural language and should stay that way. The original goal of this RFC — deterministic checks for the structural layer without disturbing the prose layer — has shipped. What stays open is whether, as the skill count grows, to graduate the structural layer from validated markdown into structured manifests or a typed DSL.

## What shipped (Option 1)

The framework lint engine validates the structural layer through `specify lint framework`, the authoring-standards surface over the `augentic/specify` repo. Skill checks are `CORE-*` rules resolved by a single generic dispatcher in two roads:

- **Road A — declarative hints** interpreted over a `WorkspaceModel` (frontmatter `schema`, `reference-resolves`, `cardinality`, `field-grammar`, `presence`, …). All policy lives in the rule's `config:`, never in the engine.
- **Road B — name-resolved in-process checkers** for cross-file rules: `skill-body`, `links-registry`, `marketplace`, `scenarios`, `prose`, and `rules`, under [`src/runtime/commands/lint/framework_tools/`](https://github.com/augentic/specify-cli).

The original Option 1 checklist is covered:

- **Frontmatter schema** — `schemas/authoring/skill.schema.json` pins `name` / `description` / `argument-hint` / `allowed-tools`; `CORE-*` rules enforce the 200/45/512 caps, the description and argument-hint grammar, and that the skill name matches its directory.
- **Reference resolution** — the `links-registry` checker resolves every `references/` link and registry link.
- **Marketplace consistency** — the `marketplace` checker cross-checks every plugin against the marketplace manifest and `schemas/authoring/marketplace.schema.json`.
- **Skill-body discipline** — the `skill-body` checker enforces body structure and directive grammar.
- **Scenario / prose / rules** — companion checkers validate the eval catalog (`schemas/authoring/scenario.schema.json`), prose house style, and the rules tree itself (`schemas/rules/rule.schema.json`).

`check.ts` (the original Deno validation script) and the once-proposed `specdev`-named binary and `check::` Rust modules never shipped under those names — the generic dispatcher replaced them. `specify lint framework` is enforced by `make lint` locally and by the framework CI job.

## What remains open

The structural layer still lives in markdown frontmatter, *validated* rather than *typed*. Two graduations stay parked behind a skill-count growth trigger.

### Option 2: YAML skill manifests (moderate friction)

Extract the structural metadata into a companion `manifest.yaml` per skill, validated by JSON Schema. The manifest declares arguments, references, tool allow-lists, authority levels, and cross-skill directives as structured data; the SKILL.md prose stays hand-authored. Framework lint cross-checks that the manifest and the SKILL.md frontmatter agree. This separates the two layers explicitly with low authoring friction.

### Option 3: Rust DSL that compiles to SKILL.md (high investment)

Model the structural skeleton in Rust — typed structs for skills, enums for tools and authority levels, `include_str!` for prose blocks — with a build step that fails on broken references, misspelled tool names, or phase-dependency cycles. This pays off only when skills must compose programmatically (e.g. generating variant skills from a base definition) or the skill count makes manual consistency impractical.

## Recommendation

Option 1 is done; extend the structural-validation surface by adding `CORE-*` rules, not by re-opening this design. Revisit Option 2 if the skill count grows past ~20 and structural drift becomes recurring, and Option 3 only when skills need to compose programmatically. Until then this stays parked.

## References

- [`specify lint framework`](https://github.com/augentic/specify-cli) — the authoring-standards surface hosting the shipped Option 1 checks.
- [Standards layer (explanation)](../../docs/explanation/standards-layer.md)
- [docs/contributing/checks.md](../../docs/contributing/checks.md) — the Road A / Road B extension model for `CORE-*` rules.
- [docs/standards/skill-authoring.md](../../docs/standards/skill-authoring.md) — the authored house style the checks enforce.
