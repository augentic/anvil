---
id: UNI-022
title: Ignore Directive Missing Rationale
severity: important
trigger: A `specify-ignore` directive omits its rationale or carries a rationale shorter than 16 characters.
---

## Rule

Every in-source `specify-ignore` directive must name a rule id and carry a non-empty rationale that explains, in human terms, why the finding is being tolerated at that location. Suppressing a finding without explanation strips the next reader of the context they need to decide whether the directive still applies, and undermines the operator policy that directives are a deliberate exception rather than a silent mute.

The 16-character minimum on the rationale is the floor at which a rationale is long enough to be useful to a reviewer; shorter rationales are accepted by the parser but reported under this rule so the operator can replace them with a real explanation. This is a workflow-hygiene rule: it ships at `important` because every unrationaled directive is a small but durable knowledge gap, and accumulating them is how a project drifts from "we acknowledge this exception" to "we forgot why this is here."

## Look For

- A directive of the form `specify-ignore: <RULE-ID>` with no following em-dash (or `--`) and rationale.
- A directive with the delimiter present but the rationale text empty or whitespace-only.
- A rationale shorter than 16 characters (for example, `bug`, `fix later`, `todo`).
- Rationales that paraphrase the directive itself rather than explaining why the finding is tolerated (for example, `ignore this`).
- Drive-by suppression added in the same change as the protected code with no commit-message or in-line rationale context.

## See Also

- [Ignore directives reference](../../../../../docs/reference/ignore-directives.md) — full grammar, comment-style table, and exit semantics.
