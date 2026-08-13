<div class="hero">
<div class="eyebrow">Understanding Emery</div>
<h1 class="hero-title">Shape of the system</h1>

Four pictures of what Emery is. The runtime hosts guests and satisfies typed effects. Source adapters, target adapters, and the engine are peers on that runtime. Durable artifacts outrank chat.

<div class="meta-row">

<span class="meta-chip"><strong>Read time</strong> ~6 min</span>

<span class="meta-chip"><strong>Depth</strong> Map</span>

</div>

</div>

<div class="audience-grid">

<div class="audience">
  <div class="who">Operator</div>
  <div class="path">
<a href="#who-talks-to-whom">Context</a> → <a href="#what-flows-through-a-change">Change flow</a> → <a href="concepts.md">Concepts</a>
  </div>
</div>

<div class="audience">
  <div class="who">Adapter author</div>
  <div class="path">
<a href="#what-runs-at-runtime">Runtime</a> → <a href="#what-flows-through-a-change">Change flow</a> → <a href="adapter-anatomy.md">Anatomy</a>
  </div>
</div>

<div class="audience">
  <div class="who">Contributor</div>
  <div class="path">
<a href="#what-runs-at-runtime">Runtime</a> → <a href="#how-the-code-is-stacked">Crates</a> → <a href="../contributing/cli-architecture.md">CLI architecture</a>
  </div>
</div>

</div>

This page is a map. It does not teach you to plan, refine, or execute a change, or to author an adapter. Those live in [Core concepts](concepts.md), [The layered stack](layered-stack.md), and [Anatomy of an adapter](adapter-anatomy.md).

The standing thesis, compressed:

> Typed boundaries. The runtime knows only effects. Control is deterministic; judgment is a typed call. Handles cross the seam, not corpora.

## Who talks to whom

<div class="pipeline">

![Who talks to whom](../assets/diagrams/system-shape/context.svg)

<p class="pipeline-caption">Operator → emery binary → engine guest → source adapters, target adapters, and the model. Artifacts persist on disk.</p>
</div>

`/emery:*` skills are thin wrappers: each invokes one `emery` command and relays its output. The binary is Omnia compiled with Emery-specific guests. The engine guest owns workflow. Adapters own specialist operations. The model backend is a host effect (`wasi-model.eval`), not a conversational partner with lifecycle authority.

## What runs at runtime

<div class="pipeline">

![What runs at runtime](../assets/diagrams/system-shape/runtime.svg)

<p class="pipeline-caption">Engine and adapter guests are peers. Omnia instantiates a fresh guest per call and satisfies filesystem, journal, workspace, and model effects.</p>
</div>

The engine guest is embedded in the binary. Source and target adapters are admitted by identity on first dispatch. Guests hold no memory between calls. Persistent state lives in host services — the snapshot store, the journal, the private workspace — so the same guests can run against local files or swapped cloud backends.

Guest-to-guest calls are host-mediated: the engine names an adapter id, the host instantiates that guest, and typed WIT records cross. There is no ahead-of-time composition of engine plus adapters into one module.

## What flows through a change

<div class="pipeline">

![What flows through a change](../assets/diagrams/system-shape/flow.svg)

<p class="pipeline-caption">Plan surveys sources into a topology. Refine extracts evidence and synthesizes specs. Execute builds in a private workspace and merges. Empty cells are intentional: that role is silent in that stage.</p>
</div>

Read the grid as three stages and three roles. Source adapters speak at plan and refine (`survey`, `extract`). Target adapters speak at refine and execute (`guidance`, then the engine-driven build loop, then `merge`). The engine is in every row: it reconciles leads, synthesizes specs, and owns order, repair budgets, and gates.

Two review pauses sit between the rows — topology after plan, specifications after refine. Invoking `emery plan execute` opens the authorization epoch. An automation may run the stages back to back; the pauses are opportunities, not attestations.

## How the code is stacked

<div class="pipeline">

![How the code is stacked](../assets/diagrams/system-shape/crates.svg)

<p class="pipeline-caption">Binary and transport on top; change and slice in the middle; project then the leaf crates. Adapters ship in augentic/emery-adapters.</p>
</div>

`change` owns the plan loop. `slice` owns extract, synthesis, build, and merge. Both sit on `project` (plan model, journal, workspaces, adapter resolver). `transport` is the typed CLI and HTTP surface. `guest` and `native` are two providers over the same engine crates; `launcher` is native-only deployment policy.

This is the same layering as [The layered stack](layered-stack.md), cut by crate instead of by invocation. Contributor rules for that layering live in [Architecture standards](../standards/architecture.md).

> [!NOTE]
> **Not on this map.** How to run a change, how to author an adapter, the RFC-104 definition home (architecture of a *surveyed client system*), and crate coding standards. Diagram Mermaid sources sit beside each SVG under `docs/assets/diagrams/system-shape/`.

<div class="see-also">
<strong>See also</strong>

- [Core concepts](concepts.md) — vocabulary and the change rhythm
- [The layered stack](layered-stack.md) — invocation layers 0–2
- [Anatomy of an adapter](adapter-anatomy.md) — source and target roles
- [CLI architecture](../contributing/cli-architecture.md) — binary, launcher, dispatch
- [Emery on Omnia](../../rfcs/architecture.md) — standing runtime thesis
</div>
