---
id: CORE-057
title: CLI Contract Drift
severity: important
trigger: Documentation cites a `specify` verb, flag, journal event id, or error discriminant the pinned CLI binary does not declare.
rule_hints:
  - kind: path-pattern
    value: "docs/**/*.md"
  - kind: path-pattern
    value: "plugins/**/*.md"
  - kind: path-pattern
    value: "adapters/**/*.md"
  - kind: path-pattern
    value: "AGENTS.md"
  - kind: path-pattern
    value: "!adapters/shared/rules/**"
  - kind: cli-contract
    value: invocations
    description: Walk every `specify …` command line in bash/sh fences and inline code spans against the binary-injected verb tree; unknown subcommands and undeclared `--flags` are findings.
    config:
      langs:
        - bash
        - sh
      # Documented-ahead verbs, exempt until they ship:
      # - `catalog` — component-catalog inference (`specify catalog infer`, components.md)
      # - `vectis`  — host-side template-updater workflow (`specify vectis update-versions`, known-drift.md)
      ignore:
        - catalog
        - vectis
  - kind: cli-contract
    value: event-ids
    description: Dotted-kebab inline code spans and `"event"` JSON fields in fenced bodies, within the contract's own event-id families, must be journal event ids the binary declares.
    config:
      json-fields:
        - event
      # Family-named tokens that are not event ids: `cli.path` is the
      # `Specify.toml` TOML key path; `plan.yaml.*` are dotted paths
      # into the plan artifact and its fixture fragments.
      ignore:
        - cli.path
      allow-prefixes:
        - "plan.yaml."
      ignore-suffixes:
        - ".yaml"
        - ".yml"
        - ".json"
        - ".jsonl"
        - ".md"
        - ".lock"
        - ".fragment"
  - kind: cli-contract
    value: error-codes
    description: Kebab-case `"error"` JSON field values in fenced bodies must be error discriminants the binary declares.
    config:
      json-fields:
        - error
---

## Rule

Documentation in this repository cites the `specify` CLI constantly — command lines in fenced `bash` blocks, verbs and flags in inline code, journal event ids, and the kebab-case `error` discriminants skills branch on. Every one of those citations is a contract claim, and until now nothing checked them: a renamed verb, a retired flag, or a reworded event id left stale prose behind with no failing check.

This rule closes that seam. The running binary injects its own machine-readable contract — the same payload `specify contract dump` emits: the clap verb tree with per-verb flags, the journal event-id taxonomy, and the wire-stable error discriminants — and the `cli-contract` hint kind checks the documentation against it:

- **`invocations`** — every `specify …` command line found in `bash` / `sh` fences or inline code is walked down the verb tree. A kebab token where a subcommand is required but not declared, or a `--flag` the resolved verb (and its ancestors) does not accept, is a finding. Positionals, placeholders (`<slice>`, `$VAR`), and everything after a literal `--` are skipped.
- **`event-ids`** — dotted-kebab inline code spans (e.g. a journal event id cited in prose) and `"event"` field values in fenced JSON must be event ids the binary declares. File names are exempted via the `ignore-suffixes` policy; YAML field paths that are not event ids belong in `ignore`.
- **`error-codes`** — kebab-case `"error"` field values in fenced JSON examples must be error discriminants the binary declares.

Named-test citations (`tests/….rs` claims against the binary's build-time test inventory) are the fourth selector of the same kind, scoped separately by [CORE-060](CORE-060-cli-test-citation-drift.md) — adapter references legitimately describe downstream generated-crate `tests/` layouts that are not CLI tests.

Because the contract is rebuilt from the binary on every `make lint` run, bumping the CLI pin re-checks every citation in the same change — the rename sweep the CLI repo's AGENTS.md rule 5 prescribes is now machine-enforced on this side.

## Look For

- A fenced `bash` example invoking a verb that was renamed or never shipped.
- An inline `specify … --flag` citation where the flag was retired.
- Prose citing a journal event id that no longer exists in the closed taxonomy.
- A fenced JSON envelope example carrying an `error` discriminant the binary never emits.

## Fix

Align the citation with the live CLI surface: run `specify contract dump --format json` (or `specify <verb> --help`) to see the declared verbs, flags, event ids, and error discriminants, and rewrite the stale citation. If the citation is intentionally illustrative — a deliberately hypothetical verb or a YAML dotted path that resembles an event id — add the offending token to the rule's `ignore` / `ignore-suffixes` config rather than weakening the prose.
