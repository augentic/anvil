---
id: CORE-058
title: Readme Vocabulary Drift
severity: important
trigger: The README vocabulary cheat sheet's `Workflow nouns` subsection is missing or its body is no longer byte-identical to the canonical `AGENTS.md` §Workflow nouns section, so the repo's most-read page restates the lifecycle nouns with drifted wording.
rule_hints:
  - kind: content-digest-eq
    value: markdown-section
    config:
      path: README.md
      section: Workflow nouns
      canonical-path: AGENTS.md
      canonical-section: Workflow nouns
    description: Hash the body under `README.md` § `Workflow nouns` and the body under `AGENTS.md` § `Workflow nouns` (leading/trailing blank lines trimmed); a digest mismatch or a missing section on either side is a finding located at the pinned section.
---

## Rule

`AGENTS.md` §Workflow nouns is the canonical home for the two lifecycle nouns (*slice*, *change*); every other surface links to it or carries a one-line summary. The one deliberate exception is `README.md`'s vocabulary cheat sheet — the repo's most-read page restates the nouns verbatim for first-contact readers. An unpinned verbatim copy is exactly the drift surface the canonicalization policy forbids, so the cheat sheet's `### Workflow nouns` subsection is digest-pinned: its body must stay byte-identical (modulo leading/trailing blank lines) to the canonical section's body.

The deterministic-hint interpreter locates both sections through the `markdown_section` facts the indexer already produced, reads the two bodies, and compares SHA-256 digests. A digest mismatch is a finding located at the README subsection; a missing section on either side is also a finding — the pin must fail loudly rather than pass vacuously, because nothing else guards the subsection's presence.

## Look For

- An edit to `AGENTS.md` §Workflow nouns (a reworded bullet, a new noun) that did not carry the same bytes into `README.md`'s cheat sheet — or the reverse, a README-only "clarification".
- A README restructure that renamed or dropped the `### Workflow nouns` subsection while the cheat-sheet heading survived.

## Fix

Edit the canonical `AGENTS.md` §Workflow nouns section first, then copy the exact body into `README.md`'s `### Workflow nouns` subsection (the prose between the heading and the next heading, surrounding blank lines excluded). If the README needs additional first-contact framing, put it under the `## Vocabulary cheat sheet` heading *before* the pinned subsection, never inside it.
