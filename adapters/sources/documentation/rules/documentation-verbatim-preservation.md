---
id: SRC-001
title: Documentation Evidence Preserves Verbatim Source Text
severity: important
trigger: A documentation-authority claim's statement, criterion, or decision text is paraphrased or summarised rather than quoted from the cited source path.
applicability:
  adapters: [documentation]
references:
  - label: documentation.extract determinism rules
    path: adapters/sources/documentation/briefs/extract.md
---

## Rule

Operator-provided documentation is the highest-fidelity input for `authority: documentation` Evidence; rephrasing it inside a claim silently weakens the authority hierarchy (`intent > documentation > behaviour`) and breaks reconciliation audit at synthesis time, because `spec.md` requirement bodies cite the claim text but reviewers verify it against the path anchor. `requirement.statement`, `criterion.criterion`, and `decision.decision` MUST quote the text at the cited `path` anchor. Light grammatical normalisation — capitalisation of the leading character, terminal punctuation, trimming surrounding whitespace — is permitted; reordering clauses, substituting vocabulary, collapsing bullet lists, or summarising prose is not. When a span of source text cannot be carried as a single behavioural claim, emit a `section` claim that preserves the bounded prose rather than paraphrasing it into a `requirement` or `criterion`.

## Look For

- Claim text that is materially shorter than, longer than, or stylistically distinct from the lines at the cited `path` anchor (`<path>#L<n>` or `<path>#L<start>-L<end>`).
- Bullet lists in the source flattened into one criterion sentence rather than one `criterion` claim per bullet.
- Synonyms or rewrites of operator vocabulary (`user` → `customer`, `must` → `should`, domain terms paraphrased into generic English).
- `requirement` or `criterion` claims whose `path` anchor points at lines that do not contain the quoted text.
- Sentences invented to "fill in" an empty section instead of returning `claims: []` for an unresolvable lead.

## Spec Guidance

When the documentation phrasing is genuinely ambiguous, contradictory, or silent, mark the resulting `spec.md` requirement with the appropriate tag (`[unknown]`, `[conflict]`, `[divergence]`) rather than smoothing the wording inside Evidence. Authority ordering depends on faithful provenance, not editorial polish — reconciliation will surface the gap to the operator, and a verbatim section claim plus a tagged requirement is always preferable to a confident paraphrase.
