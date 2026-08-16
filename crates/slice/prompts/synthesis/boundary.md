# Proceed vs boundary-escalation

Every synthesis answer carries `kind: proceed` or `kind: boundary-escalation`, plus a closed five-dimension `assessment` (integers 0–10): `behavioural-breadth`, `coupling`, `uncertainty`, `context-volume`, `verification-surface`. The engine scores the weighted sum against the bound target's pinned slice-split threshold.

Complexity is a trigger, not sole authority over lifecycle shape. A score above the threshold causes escalation only when the Evidence supports separately acceptable behavioural boundaries or shows that the complete slice exceeds the target's bounded verification envelope. If the work remains one coherent acceptance unit, emit `proceed`.

## `proceed`

Write the change-artifact bundle (`model.yaml`, prose files, `specs/<domain>/spec.md`, optional `decisions/<slug>.md`) into your working tree and answer with the envelope. The engine validates the staged tree, persists it, stamps `refined`, and writes `refinement.yaml`.

## `boundary-escalation`

Write **nothing** into the working tree. Name the affected terminal `(source, lead)` pairs already bound on this leaf in `affected[]`, and say why the Evidence supports a split (or an over-envelope leaf) in `rationale`.

The engine does not promote artifacts, does not stamp `refined`, and does not start build work. It runs focused survey for the named parents and nearest-domain re-decomposition into an **inert** amendment proposal. Live `leads.md` / `decomposition.yaml` / `plan.yaml` stay unchanged until the operator applies that proposal.

`affected[]` must be this leaf's bound terminals — not hypothetical child leads. The kernel rejects an empty list, an empty rationale, an unknown pair, and a score at or below the slice-split threshold.
