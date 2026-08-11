---
name: code-review
description: >-
  Careful code-quality sweep of the Emery repository (or a given crate/directory)
  against AGENTS.md and docs/standards/. Use when the user invokes /code-review
  or asks for a standards-backed quality review of Rust workspace code.
disable-model-invocation: true
---

# code-review

Do a careful code-quality sweep of the current repository (or the crate/directory
given as an argument, if any).

Before reviewing, read the repo's own standards: `AGENTS.md` and everything it
links under `docs/standards/` (style, coding-standards, testing). Review
*against* those documents — they outrank your general preferences.

Look for:

1. Code that can be simplified, rationalised, or removed outright.
2. Non-idiomatic Rust that could use recognisable patterns and idioms.
3. Names longer than needed. Heuristic: >15 chars is suspect, >25 needs
   justification. Sharper rule: the module path is context — flag
   `show_registry` in `registry.rs`, which should be `registry::show`.
4. Unit tests that violate the integration-first policy: any `src`
   `#[cfg(test)]` test whose behavior is reachable through the public surface.
5. YAGNI — abstractions, flags, or generality with no current consumer.
6. Latent bugs and footguns.

Process:
- Partition by workspace crate; use one explore subagent per crate if helpful.
- Each area returns at most its top 5 findings — prioritize, don't enumerate.

Rules of evidence:
- Every finding cites file and line.
- "Unused / can be removed" claims require a search showing no callers,
  including prose (`docs/`, `AGENTS.md`, adapter repos where relevant).
- Skip purely stylistic preferences with no standards backing.
- If something might be a contract-locked seam rather than YAGNI, flag the
  uncertainty instead of asserting.

Output: report only — make no edits. Rank findings by value. For each:
location, one-line problem, proposed change, estimated effort and risk.
