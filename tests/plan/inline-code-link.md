# Inline-Code Link Fixture

This file is a regression fixture for `scripts/checks/links.ts::checkMarkdownLinks`.

The link checker strips fenced code blocks before scanning for relative
`[text](path)` links, but markdown also has *inline* code spans delimited by
single backticks. A markdown link that lives inside such a span should be
treated as code, not as a real link, and must not be resolved against the
filesystem.

The lines below intentionally embed broken paths inside single-backtick
spans. If `make checks` exits 0, the inline-code stripping is working; if it
fails with `Broken link`, the predicate has regressed.

- Sample 1: `[Phase outcome contract](../../references/phase-outcome-contract.md)`
- Sample 2: `[RFC-N](rfcs/...)`
- Sample 3: `[broken target](this-path-does-not-exist.md)`

This file deliberately omits YAML frontmatter so the scenario-frontmatter
check skips it (per `tests/plan/README.md`: "Prose-only documents ... are
skipped by the scenario frontmatter check").
