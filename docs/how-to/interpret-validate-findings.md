<div class="hero">
<div class="eyebrow">How-to</div>
<h1 class="hero-title">Interpret validate findings</h1>

Read the `DiagnosticReport` that `emery plan validate` and `emery slice validate` emit, and decide what actually blocks you.

<div class="meta-row">

<span class="meta-chip"><strong>Verbs</strong> plan validate · slice validate</span>

<span class="meta-chip"><strong>Blocking</strong> exit 2</span>

</div>

</div>


<div class="when">
<strong>When to use.</strong>

Use this guide when a validate verb exits 2, when `emery plan execute` reports `stuck` (plan validate is the first triage step), or when you want to know whether a finding is safe to ignore.
</div>


<section id="report-shape" markdown="1">

<h2><span class="num">1</span> The report shape</h2>

Both verbs render the same neutral `DiagnosticReport` envelope (`{ version, summary, findings }`; add `--format json` for the structured form). Each finding carries:

| Field | Meaning |
| ----- | ------- |
| `rule-id` | Kebab-case discriminant, e.g. `cycle-in-depends-on`, `slice-model-source-orphan` — grep-stable, part of the public contract |
| `severity` | `critical` / `important` / `suggestion` / `optional` |
| `kind` | `violation` (structural defect) or `review` (a deterministically-raised request for agent judgment) |
| `impact` | Human-readable message |
| `slice` | The plan entry or slice the finding is scoped to (optional) |
| `evidence` | A plain `snippet`, or `{ "kind": "structured", "data": … }` for machine-readable payloads |
</section>


<section id="what-blocks" markdown="1">

<h2><span class="num">2</span> What blocks and what doesn't</h2>

The blocking rule is severity × kind: **open `critical` or `important` violations block the gate and exit 2**. Everything else is advisory:

- `suggestion` and `optional` findings never block — exit stays 0.
- `kind: review` findings never block regardless of severity — they flag places where agent judgment was requested (e.g. thin discovery-lead synopses), not defects.

So an exit 0 with findings is normal: read the suggestions, act on what's worthwhile, move on. An exit 2 names at least one blocking violation to fix before the lifecycle gate opens.
</section>


<section id="common-plan-findings" markdown="1">

<h2><span class="num">3</span> Common plan findings</h2>

| `rule-id` | Severity | Recovery |
| --------- | -------- | -------- |
| `cycle-in-depends-on` | important | Break the cycle: `emery plan amend <entry> --depends-on …` |
| `duplicate-source-key` | important | A slice binds at most one lead per source — re-bind with `--sources <key>=<other-lead>` |
| `orphan-source` | suggestion | A declared source no entry references — bind it or remove the declaration |

See [emery plan validate](../reference/cli/plan.md#emery-plan-validate) for the full table.
</section>


<section id="common-slice-findings" markdown="1">

<h2><span class="num">4</span> Common slice findings</h2>

| `rule-id` | Meaning | Recovery |
| --------- | ------- | -------- |
| `slice-spec-provenance-stale` | A kernel-rendered `ID:` / `Sources:` / `Status:` line was hand-edited | Revert the edit; drive resolution through overrides and re-refine — see [Resolve spec conflicts](resolve-spec-conflicts.md) |
| `slice-model-source-orphan` | `model.yaml` cites a source the plan no longer binds | Re-run `emery plan execute` after the plan amendment — the drifted slice re-refines |
| `slice-authority-override-orphan-source` | An authority override names a source key the slice doesn't bind | Fix the override: `emery plan amend <entry> --authority-override <kind>=<source>` |
| `slice-model-schema` | `model.yaml` fails its typed schema | Re-run `emery plan execute`; never hand-edit `model.yaml` |

The drift family (`slice-model-*`) shares one theme: `spec.md` and `model.yaml` must agree, and the fix is re-running the synthesis that writes both — not editing either by hand.
</section>


<div class="see-also">
<strong>See also</strong>

- [emery slice validate](../reference/cli/slice.md#emery-slice-validate) — the slice check catalogue
- [CLI output shapes](../reference/cli-output-shapes.md#emery-plan-validate) — the JSON envelope
- [Resolve spec conflicts](resolve-spec-conflicts.md) — the kernel-rendered lines rule
</div>
