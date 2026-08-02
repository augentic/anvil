# Anatomy of an Adapter

Emery has two adapter roles with a shared shape. **Source adapters** turn external material (operator intent, written documentation, legacy code, screenshots) into structured `Evidence`. **Target adapters** turn that evidence into code by guiding core synthesis and driving build / merge. The role you are authoring decides which operations you implement; the on-disk shape is the same.

> [!NOTE]
> This page explains the roles and how they fit together. The precise authoring contract — metadata fields, claim kinds, the sandbox table, resolver mechanics, and the author checklist — lives in the [Adapter contract](../reference/adapter-contract.md) reference. If you just want to understand *how an adapter fits into a change* (a legacy-code source surveys a lead, extracts evidence, and a target turns the resulting spec into code), read [From sources to slices](reconciliation.md) first.

<div class="audience-grid">
  <div class="audience">
    <div class="who">Source author</div>
    <div class="path"><a href="#two-roles-one-shared-shape">Roles</a> → <a href="../reference/adapter-contract.md#source-adapter-contract">survey + extract contract</a></div>
  </div>
  <div class="audience">
    <div class="who">Target author</div>
    <div class="path"><a href="#two-roles-one-shared-shape">Roles</a> → <a href="../reference/adapter-contract.md#target-adapter-contract">guidance + build + merge contract</a></div>
  </div>
  <div class="audience">
    <div class="who">Operator</div>
    <div class="path"><a href="#authority-resolution">Authority resolution</a> → <a href="../reference/cli/adapter.md">CLI resolve</a></div>
  </div>
</div>

## Two roles, one shared shape

| Axis     | Role   | Operations                   | Default examples                                       | Lives under                |
| -------- | ------ | ---------------------------- | ------------------------------------------------------ | -------------------------- |
| `source` | input  | `survey`, `extract`          | `intent`, `documentation`, `typescript`, `screenshots` | `sources/<name>/` |
| `target` | output | `guidance`, `build`, `merge` | `omnia`, `vectis`, `contracts`                         | `targets/<name>/` |

Both ship as a single WebAssembly component exporting the matching axis interface from the closed WIT contract (`wit/emery.wit`) — one component, no manifest file. The shared shape is the **plugin** (a vocabulary noun for the audience tag, not the Rust module name) — same component contract, same prose layout in the authoring repo. The axis decides the operations, which derive from the WIT contract rather than being declared on the wire.

The WIT package also defines an additive combined `adapter` world (`export source; export target;`) so one component can serve both axes. It exists for self-contained testing — Emery's own mock adapter uses it — and imposes no obligation on external adapters: source-only and target-only components remain the published shape.

## How the roles meet the loop

A source adapter participates twice: `survey` at plan time (one [lead](../appendices/glossary.md#l) per slice-sized unit into `discovery.md`) and `extract` at slice time (one `Evidence` document per bound source). A target adapter participates three times per slice: its `guidance` prompt is read into synthesis context at refine, its `build` operation generates the code, and its `merge` operation gates the landing. The engine owns everything between — lifecycle, artifact schemas, synthesis, and state transitions — so an adapter contributes specialist behaviour without ever driving the workflow.

```text
/emery:plan    →  source.survey     (leads into discovery.md)
/emery:refine  →  source.extract    (evidence/<source>.yaml)
               →  target.guidance   (idiom guidance for synthesis)
/emery:build   →  target.build      (code generation)
/emery:merge   →  target.merge      (preflight/postflight landing gates)
```

For the exact operation signatures, the lead and claim grammars, the sandbox posture, and the resolver mechanics, see the [Adapter contract](../reference/adapter-contract.md).

## Authority resolution

Authority hierarchy is a property of the adapter, not of a slice. Source adapters declare which authority class their evidence carries (`intent` > `documentation` > `behaviour`); core synthesis uses the class to resolve disagreements between two `Evidence` rows for the same claim. Operators override per-slice at Gate 1 via `emery plan amend <entry> --authority-override <claim-kind>=<source>` and then re-run `/emery:refine` — never by hand-editing the rendered `spec.md` provenance lines (see [Resolve spec conflicts](../how-to/resolve-spec-conflicts.md)). Normative detail lives in [`crates/slice/prompts/synthesis/authority.md`](../../crates/slice/prompts/synthesis/authority.md); the operator-facing walk-through is in [From sources to slices](reconciliation.md).

<div class="authority-widget">
  <h4>Resolution flow — click a scenario</h4>
  <p style="font-size: 13px; margin: 0 0 10px;">
    When two claims of the same kind disagree, synthesis walks three steps in order. Pick a scenario to see which step fires.
  </p>
  <div class="auth-controls" id="auth-ctl">
    <button type="button" class="on" data-scenario="slice">Per-slice override set</button>
    <button type="button" data-scenario="default">No override, classes differ</button>
    <button type="button" data-scenario="tied">All same authority class</button>
  </div>

  <div class="auth-flow" id="auth-flow">
    <div class="auth-step" data-step="1">
      <div class="n">1</div>
      <div class="label">
        <strong>Per-slice <code>authority-override.&lt;kind&gt;</code></strong>
        <div class="desc">Matches a contributing source key → that source wins.</div>
      </div>
      <div class="verdict">slice winner</div>
    </div>
    <div class="auth-step" data-step="2">
      <div class="n">2</div>
      <div class="label">
        <strong>Default ordering</strong>
        <div class="desc"><code>intent &gt; documentation &gt; behaviour</code> on document-level <code>authority:</code>.</div>
      </div>
      <div class="verdict">default winner</div>
    </div>
    <div class="auth-step" data-step="3">
      <div class="n">3</div>
      <div class="label">
        <strong>Still tied</strong>
        <div class="desc"><span class="pill conflict">conflict</span> Status: <code>conflict</code> + <code>[conflict]</code> inline tag.</div>
      </div>
      <div class="verdict">no winner</div>
    </div>
  </div>
  <p style="font-size: 12px; margin: 8px 0 0; font-family: ui-monospace, monospace;">
    Every step that does not fire is consulted and skipped; the chain is byte-stable. Inspect outcomes with <code>emery slice provenance</code> (projected on demand from <code>model.yaml</code>).
  </p>
</div>

## Adapter manifests vs Cursor plugin manifests

Cursor and `emery` are different runtimes. They share no fields, no loader, and no discovery path.

- **`.cursor-plugin/plugin.json`** (under `plugins/<name>/` in this repo) is read by Cursor itself to register IDE surface area: `/emery:*` skills, rules, and slash commands. It is invisible to the `emery` CLI.
- **The adapter component** lives in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters) and is resolved by the `emery` CLI through the provider's `adapter::Resolver` capability; the shipped WASI provider delegates to `resolver::Component`, and metadata is the component's own `metadata` answer. Cursor never consults it.

Neither system references the other, neither loader probes for the other, and neither cache is shared. If you are answering "is there a JSON config for adapters?": no — adapters have no manifest file at all; `plugin.json` is Cursor's, not Emery's.

## See also

- [Adapter contract](../reference/adapter-contract.md) — the precise authoring contract: identity, metadata, operations, sandboxing, resolver, checklist
- [From sources to slices](reconciliation.md) — how leads and evidence become slices and specs
- [Source adapters](../reference/sources/index.md) / [Target adapters](../reference/targets/index.md) — per-adapter catalogs
