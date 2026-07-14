<div class="hero">
<div class="eyebrow">Specify Developer Guide</div>
<h1 class="hero-title">Governed AI development starts with the spec</h1>

Specify turns AI-assisted delivery into a repeatable docs-first workflow. Plans, slice artifacts, behavioral requirements, designs, and implementation tasks stay version-controlled so every generated change has a trail from intent to merge.

<div class="meta-row">

<span class="meta-chip"><strong>Version</strong> 2.0</span>

<span class="meta-chip"><strong>Status</strong> Released</span>

<span class="meta-chip"><strong>Rhythm</strong> plan → review → execute → finalize</span>

</div>

</div>

<div class="proof-strip">
  <div class="proof-item">
    <div class="proof-kicker">Scope</div>
    <div class="proof-value">Plan first</div>
    <div class="proof-copy">A change is defined before implementation starts, even when it contains a single slice.</div>
  </div>
  <div class="proof-item">
    <div class="proof-kicker">Control</div>
    <div class="proof-value">Gate 1</div>
    <div class="proof-copy">The operator reviews and stamps the plan before <code>specify plan execute</code> can run.</div>
  </div>
  <div class="proof-item">
    <div class="proof-kicker">Trace</div>
    <div class="proof-value">Artifacts</div>
    <div class="proof-copy">Specs, designs, tasks, evidence, and merges remain auditable after the chat is gone.</div>
  </div>
</div>

## The workflow rhythm

<div class="rhythm">
  <div class="rhythm-step">
    <div class="rhythm-num">01</div>
    <div class="rhythm-label">Plan</div>
    <div class="rhythm-title">Define the change</div>
    <p>Bind sources, survey leads, and produce <code>change.md</code>, <code>plan.yaml</code>, and <code>discovery.md</code>.</p>
  </div>
  <div class="rhythm-step">
    <div class="rhythm-num">02</div>
    <div class="rhythm-label">Review</div>
    <div class="rhythm-title">Lock the scope</div>
    <p>The operator stamps the plan <code>approved</code>; the agent does not approve its own scope.</p>
  </div>
  <div class="rhythm-step">
    <div class="rhythm-num">03</div>
    <div class="rhythm-label">Execute</div>
    <div class="rhythm-title">Refine, build, merge</div>
    <p>Each slice moves through evidence, spec, design, tasks, implementation, and merge.</p>
  </div>
  <div class="rhythm-step">
    <div class="rhythm-num">04</div>
    <div class="rhythm-label">Finalize</div>
    <div class="rhythm-title">Close the trail</div>
    <p>Branches, PR state, and archive steps close the change after every entry is done.</p>
  </div>
</div>

## Choose your path

<div class="card-grid">
  <a class="card" href="tutorials/quick-start.md">
    <div class="card-head">
      <h3 class="card-title">Run Specify for the first time</h3>
      <span class="card-time">Start</span>
    </div>
    <div class="card-body">
      <p>Install prerequisites, initialize a project, and drive your first docs-backed change.</p>
    </div>
  </a>
  <a class="card" href="reference/quick-reference.md">
    <div class="card-head">
      <h3 class="card-title">Look up a command</h3>
      <span class="card-time">Reference</span>
    </div>
    <div class="card-body">
      <p>Find the slash-command and CLI surfaces for the plan-driven workflow.</p>
    </div>
  </a>
  <a class="card" href="explanation/layered-stack.md">
    <div class="card-head">
      <h3 class="card-title">Understand the architecture</h3>
      <span class="card-time">Concepts</span>
    </div>
    <div class="card-body">
      <p>See how Specify, source adapters, target adapters, and downstream builds fit together.</p>
    </div>
  </a>
  <a class="card" href="how-to/drive-slice-manually.md">
    <div class="card-head">
      <h3 class="card-title">Recover when execute stops</h3>
      <span class="card-time">How-to</span>
    </div>
    <div class="card-body">
      <p>Resume the slice loop by hand when a plan parks on refine, build, or merge.</p>
    </div>
  </a>
</div>

## See it in action

```text
/spec:init omnia

/spec:plan fix-typo source intent="fix typo in user.rs"
  --> writes change.md + plan.yaml + discovery.md, exits at pending

specify plan transition fix-typo approved
  --> operator review step (Gate 1)

specify plan execute
  --> /spec:refine + /spec:build + /spec:merge per slice until drained

/spec:finalize fix-typo
  --> publish outside Specify, archive plan
```

## Built for the Augentic stack

<div class="platform">
  <div class="platform-product" data-active="true">
    <div class="platform-name">Specify</div>
    <div class="platform-role">Workflow engine</div>
    <div class="platform-body">
      <p>Owns planning, slice artifacts, lifecycle gates, and the auditable contract from intent to merge.</p>
    </div>
  </div>
  <div class="platform-product">
    <div class="platform-name">Omnia</div>
    <div class="platform-role">Runtime target</div>
    <div class="platform-body">
      <p>Consumes refined specs and designs to generate sandboxed Rust WASM services downstream.</p>
    </div>
  </div>
  <div class="platform-product">
    <div class="platform-name">Vectis</div>
    <div class="platform-role">Interface target</div>
    <div class="platform-body">
      <p>Applies the same spec-first discipline to cross-platform UI generation.</p>
    </div>
  </div>
</div>

## Guide structure

- **[Getting Started](orientation/index.md)** -- what Specify is and how to install it.
- **[Tutorials](tutorials/index.md)** -- hands-on walkthroughs from first slice to cross-repo changes.
- **[How-to Guides](how-to/index.md)** -- task-oriented recipes for common operator situations.
- **[Understanding Specify](explanation/concepts.md)** -- concepts, architecture, and design decisions.
- **[Reference](reference/index.md)** -- skills, CLI commands, artifact formats, and configuration.
- **[Contributing](contributing/index.md)** -- the Rust runtime, consistency checks, and Cursor skill wrappers.
