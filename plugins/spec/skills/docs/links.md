# Spec skill link anchors

Named anchors for cross-references that the skills under `plugins/spec/skills/` consume. Keeping the canonical RFC / brief / schema paths in one place avoids fragile multi-`..`/ hops from every SKILL and fixture file, and lets the real target move (e.g. an RFC relocating between `rfcs/` and `rfcs/archive/`) without churning every caller.

To reference an anchor from a skill file use

```markdown
[RFC-2](../docs/links.md#rfc-2)        <!-- from plugins/spec/skills/<skill>/SKILL.md -->
```

or, from a fixture two levels further down:

```markdown
[RFC-2](../../../docs/links.md#rfc-2)  <!-- from plugins/spec/skills/<skill>/fixtures/<fx>/ -->
```

Update the targets listed below — not the callers — when a linked document moves. All target paths are given relative to the **repo root**; they are plain code spans rather than markdown links so that this file itself does not contribute to the "no four-`..` hop" invariant the skills enforce.

## RFCs

<a id="rfc-1"></a>
<a id="rfc-1-cli"></a>
- **RFC-1 CLI** — repo path: `rfcs/archive/rfc-1-cli.md`.

<a id="rfc-1-plan"></a>
- **RFC-1 plan** — repo path: `rfcs/archive/rfc-1-plan.md`.

<a id="rfc-1a"></a>
- **RFC-1a validation** — repo path: `rfcs/archive/rfc-1a-validation.md`.

<a id="rfc-2"></a>
- **RFC-2** (execution, plan authoring, layers 2 & 3) — repo path: `rfcs/archive/rfc-2-execution.md`.

  Subsections the skills cite (resolve against the RFC body):

  | Anchor | RFC-2 section |
  |---|---|
  | `#rfc-2-invariants` | "Invariants" |
  | `#rfc-2-phase-outcome-contract` | "Phase Outcome Contract" |
  | `#rfc-2-plan-mutation` | "Plan Mutation and Crash Safety" |
  | `#rfc-2-driver-concurrency` | "Driver Concurrency" |
  | `#rfc-2-output-observability` | "Output and Observability" |
  | `#rfc-2-phase-boundary-rule-2` | "Phase Boundary → Rule 2" |
  | `#rfc-2-layer-2` | "Layer 2: Automated Execution" |
  | `#rfc-2-layer-3` | "Layer 3: Plan Authoring" |
  | `#rfc-2-context-resumption` | "Context Threading → Resumption Within a Change" |
  | `#rfc-2-the-plan` | "The Plan" |

<a id="rfc-2-invariants"></a>
<a id="rfc-2-phase-outcome-contract"></a>
<a id="rfc-2-plan-mutation"></a>
<a id="rfc-2-driver-concurrency"></a>
<a id="rfc-2-output-observability"></a>
<a id="rfc-2-phase-boundary-rule-2"></a>
<a id="rfc-2-layer-2"></a>
<a id="rfc-2-layer-3"></a>
<a id="rfc-2-context-resumption"></a>
<a id="rfc-2-the-plan"></a>

## Schema briefs

<a id="omnia-discovery"></a>
- **Omnia discovery brief** — repo path: `schemas/omnia/briefs/plan/discovery.md`.

<a id="omnia-propose"></a>
- **Omnia propose brief** — repo path: `schemas/omnia/briefs/plan/propose.md`.

<a id="vectis-propose"></a>
- **Vectis propose brief** — repo path: `schemas/vectis/briefs/plan/propose.md`.
