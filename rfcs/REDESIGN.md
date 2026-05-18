# Specify redesign — concise sketch

> Status: Historical sketch — captures the high-level redesign discussion that preceded [RFC-25 (Rename `capability` → `profile`)](rfc-25-profile-rename.md). The rename was taken as a standalone step; the structural redesign sketched below is deferred and remains open for a future RFC.

I read `REVIEW.md`, `rfc-20-plan.md`, the now-deleted `rfc-25-source-profiles.md` (superseded by RFC-25), and `profiles/omnia/profile.yaml` to ground this against the current code path. Below is a single-pass redesign at the highest level; everything is open for drill-down.

## Core observation

Today's design has the wrong primary axis. It treats **Layer 1 vs Layer 2** as the organising spine and bolts source-side concerns onto target-keyed structures. RFC-25 already noticed this and proposed a parallel `source-profile` apparatus next to `profile/`. Building two parallel-but-duplicate plugin systems is the symptom; the cure is to make the **source / target distinction** the primary axis and collapse both into one extension model.

A second mis-design: `project.yaml.profile` is **singular**. A real system is often `omnia + contracts`, or `vectis + contracts`, or all three. The framework should compose, not pick one.

A third observation: once legacy migration finishes, the source axis simply has nothing plugged into it. There's no "mode" to switch — emptiness is the off state.

## Redesign in five moves

### 1. One plugin shape, two axes

Replace both `profiles/<name>/profile.yaml` and RFC-25's `source-profiles/<name>/profile.yaml` with a single concept — call it a **lens** (or module / extension; name TBD).

```yaml
# lenses/<name>/lens.yaml
name: omnia
version: 1
axis: target           # source | target
phases:                # phases the lens contributes briefs to
  define: [proposal, specs, design, tasks]
  build:  [build]
  merge:  [merge]
briefs:
  proposal: briefs/proposal.md
  ...
tools:                 # optional WASI tools the lens ships (per RFC-15)
  - id: contract
    wasm: tools/contract.wasm
detect:                # source lenses only — auto-detection rules
  - { kind: file-glob, pattern: "package.json" }
```

Source lenses contribute to `analyze | survey | extract` phases. Target lenses contribute to `define | build | merge`. Same schema, same resolver, same cache (`.specify/.cache/lenses/<name>/`), same `specify lens {resolve, list, validate}` verb family. RFC-25's intent is preserved; the duplicate scaffolding (separate schema, separate resolver, separate verb, separate cache path) is not.

### 2. Project composes a **set** of lenses, not one

```yaml
# .specify/project.yaml
lenses:
  targets: [omnia, contracts]      # what we build
  sources: []                       # populated by sources.yaml profile inference, or [] for greenfield
```

`sources` is derived from `sources.yaml` rather than hand-listed — the project file states the targets and inherits the source set from the catalogue. A pure-frontend project lists `targets: [vectis, contracts]`. A pure-backend lists `[omnia, contracts]`. A full-stack lists all three. A documentation-only product would list just `[contracts]`.

This is what kills the "Layer 1 vs Layer 2" framing as a design primitive: a layer is just a phase set, and any project may include any subset of phases.

### 3. Each phase is a **composition** of lens contributions, not a pipeline owned by one profile

Current `profile.yaml:pipeline.define = [proposal, specs, design, tasks]` is a single ordered pipeline owned by one profile. Replace with a per-phase composition rule:

- The **framework** owns the cross-target shared briefs (`proposal.md` — what & why is target-agnostic).
- Each enabled **target lens** contributes target-specific briefs to that phase (`omnia/design.md`, `vectis/design.md`, `contracts/design.md`), dispatched per target.
- Each enabled **source lens** contributes source-specific briefs to its phases (`typescript-node/analyze.md`, `cobol-mvs/analyze.md`), dispatched per source.

A slice's actual brief plan is then `framework_briefs + Σ(target_lens.briefs[phase]) + Σ(source_lens.briefs[phase])`. The slice carries metadata for which lenses it touches; a backend-only slice in a full-stack project doesn't drag `vectis` through `/spec:build`.

### 4. Discriminators become resolver calls

`/change:analyze`'s `kind: legacy-code | documentation` is a hard-coded discriminator inside the skill prose. Replace with a lens-resolver call: `documentation` becomes a source lens like any other; `legacy-code` is "auto-detect the source lens from the path". The skill body shrinks to "resolve lens, load the lens's `analyze.md`, dispatch". RFC-25 already proposes this — it's the right shape; just move it onto the unified lens resolver instead of inventing a second one.

Same treatment for `/change:survey`: RFC-20 v1.5 already has per-language enumeration briefs under `plugins/change/skills/survey/briefs/enumerate/<language>.md`. Move those into `lenses/typescript-node/briefs/survey.md` etc.; the skill resolves the source lens once and loads its `survey.md`.

### 5. CLI extensibility piggybacks on the existing WASI tool surface

The `specify tool run <name>` dispatcher (RFC-13/15) is already the right seam for CLI extension. A lens that needs deterministic CLI logic (a COBOL copybook flattener, a `package.json` parser, a contracts validator) declares its WASI tool inside its `lens.yaml`. The resolver materialises briefs and tools together — there is no separate plugin-registration path for "code that runs in the binary" vs "prose the skill reads".

This means a new source-language adoption is a single new directory:

```text
lenses/cobol-mvs/
  lens.yaml
  briefs/{analyze,survey,extract}.md
  tools/cobol-flatten.wasm    # optional
```

No code in the host binary changes. The RFC-20 F1 deletion is the right move precisely because the trait-shaped extension point belonged on the lens axis, not in the binary.

## What goes away

- The parallel `source-profile` scaffolding RFC-25 proposes (schema, resolver, verb, cache, validators) — collapsed into the unified lens model.
- `project.yaml.profile` as a singular field.
- The `kind: legacy-code | documentation` switch baked into `/change:analyze`'s skill body.
- Source-side prose inside target-keyed `briefs/<cap>/analyze.md` files.
- The "Layer 1 vs Layer 2" framing as the structural primitive (it survives as a *phase grouping* — define/build/merge vs draft/execute/finalize — but stops dictating the plugin shape).

## What stays unchanged

- Artifact contract: `proposal.md`, `spec.md`, `composition.yaml`, `design.md`, `tasks.md`, `surfaces.json`, `metadata.json`, `discovery.md`'s `## Candidate inventory` handshake.
- Single-writer rule for `plan.yaml` / `.metadata.yaml` / archive paths.
- WASI tool dispatch shape (`specify tool run <name>`).
- The bounded repair loop in `/change:survey`.
- RFC-20's pivot to agent-produced surfaces with deterministic CLI validation.

## Migration shape (rough, for drill-down)

1. Land the unified `lens.yaml` schema next to (not replacing) the existing two.
2. Move `profiles/<name>/` directories under `lenses/<name>/` with `axis: target`; symlink the old path for one release.
3. Introduce `lenses/{typescript-node, documentation, default}/` with the content RFC-25 already specifies (this is RFC-25's payload — just under the unified directory).
4. Bump `project.yaml` to support `lenses.targets: [...]` while keeping the singular `profile` field as a compat alias for one release.
5. Refactor each phase skill body to resolve lenses and iterate, instead of hard-coding "the profile".
6. Retire the old paths and the singular field in the next major.

---

## Open questions for drill-down

- **Naming.** "Lens" is one option; "module", "extension", "facet", "profile" are others. The name carries the redesign — "profile" reads as source-only, "profile" reads as target-only, neither survives the merge.
- **Brief-composition semantics.** When multiple target lenses are active in one slice, does each contribute its own `design.md` (per-target artifacts in `.specify/slices/<name>/design/<target>.md`), or do we merge into one `design.md` with target-keyed sections? The former is mechanically simpler; the latter reads better for an operator reviewing a slice.
