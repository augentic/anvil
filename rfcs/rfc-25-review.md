# RFC-25 Review — open items

> Review notes against [rfc-25-adapters.md](rfc-25-adapters.md). The structural problems posed by the original problem statement (collapse the doc/legacy bifurcation, move legacy out of core, give the doc-driven and code-driven paths symmetry) are all addressed by the current draft. What follows is the residual list — contradictions, gaps, and polish — to work through before landing.

## High-priority — internal contradiction

### 1. Who owns source → `spec.md` synthesis: target adapters or Specify core?

§Slice authoring synthesis step 3 reads:

> Synthesise: invoke the Specify artifact-synthesis contract with all N evidence packs as input. Target adapters contribute their `specs` and `design` briefs where target-specific shaping is required, but the source-to-`spec.md` / `design.md` contract is uniform and Specify-owned. The briefs are written to consume pack sets, not single packs.

But §Adapter implementation shape still has target adapters declaring `specs` and `design` as first-class capabilities with required brief paths:

```yaml
capabilities:                        # source: enumerate, extract
  - enumerate                        # target: proposal, specs, design, tasks, build, merge
  - extract
briefs:
  enumerate: briefs/enumerate.md
  extract:   briefs/extract.md
```

These statements aren't reconcilable as written. If synthesis is Specify-owned, target `specs`/`design` briefs are *contributors* (target-idiom hints, target-specific design sections), not owners — and the capability set should reflect that, e.g. by splitting into a Specify-owned `specs-synthesis` core brief plus an optional target `specs-shape` brief, or by demoting target `specs`/`design` to optional.

RFC-24's whole shape assumes target ownership of these briefs; if RFC-25 is repositioning that, the consequences for RFC-24 should be spelled out.

This is the most consequential thing to resolve before the RFC ships, because it determines what target adapters actually *do* at slice time.

### 2. What does `enumerate` mean for `documentation`?

`legacy-code-*` enumeration has a well-defined output discipline (per-language briefs, repair loop, `surfaces.json`). `intent` enumeration is trivial (one candidate from the brief). `documentation` enumeration sits in between — read N docs, propose slice-sized candidates — and the RFC currently doesn't say what shape that takes.

Open questions to answer:

- Is a candidate one document? One heading? One requirement cluster? One ADR?
- Does the documentation adapter need a structured-evidence sidecar analogous to `surfaces.json`, or is its enumeration direct (one brief read → candidate blocks)?
- What's the repair loop, if any?

Today this discipline lives implicitly in `/change:analyze`'s prose. Now that it's becoming a core-shipped source adapter, the enumeration grammar deserves at least a paragraph (and probably a §Default source adapters subsection in its own right, parallel to what `sources/legacy-code-*` will get). Otherwise `documentation` as a default reads as aspirational: "we'll ship it, the discipline TBD".

This matters because the doc-driven path is the *normal* path for greenfield projects — it's the one most users will hit first.

## Medium-priority — gaps and mismatches

### 3. Pack-level vs entry-level authority

§Source adapter contract classifies `authority` at the *pack* level:

```yaml
authority: observed-behaviour        # one of: intent | external-contract | design-spec | observed-behaviour
```

But the documentation pack example legitimately mixes authorities — `requirement-statement` (intent-flavoured), `decision-record` (design-spec), `acceptance-criterion` (intent-flavoured), `document-section` (varies). One authority class per pack means an operator who binds `docs/` ends up with everything classified `design-spec`, including statements that should outrank an `external-contract`.

Two options worth picking between explicitly:

- **Pack-level (status quo).** The documentation adapter has to split a single docs corpus into multiple bindings (`product-requirements`, `adrs`, `architecture-notes`) so each gets the right authority.
- **Entry-level.** Each evidence kind carries its own classification inside `evidence[]`; pack-level `authority` becomes the default.

§Authority hierarchy reads cleanly only if entry-level authority is what synthesis actually consumes. Open Question 7 (operator override at slice-binding time) is a workaround for the pack-level model but not a substitute for getting the granularity right.

### 4. Boundary between `documentation` (in-core) and `sources/openapi`/`asyncapi`/`json-schema` (carved out)

OpenAPI and JSON Schema *are* documentation in the broad sense — they're declarative product/API descriptions. But the RFC carves them into `specify-sources-contracts`. That's defensible (they're structured, validator-backed, contract-flavoured) but the boundary needs a sentence:

> "`documentation` consumes prose, ADRs, and unstructured/semi-structured product-intent docs; structured contract formats go through their dedicated source adapter to get parser-grade evidence."

Without this, an operator with a Markdown spec full of inline OpenAPI fragments doesn't know which adapter to bind.

### 5. Implicit vs explicit binding for the two defaults

§project.yaml shape lists both `intent` and `documentation` in the `sources:` array as if they're hand-listed, but Open Question 5 leans toward implicit-`intent` binding. The corresponding answer for `documentation` isn't stated. Three possibilities, pick one:

- Both implicit, never appear in `project.yaml`.
- `intent` implicit (always available), `documentation` explicit (only available when listed).
- Both explicit (current YAML reading).

The current text reads as the third, but the prose tone of §Default source adapters reads as the first. Worth aligning.

### 6. Where does `/change:analyze`'s prose go?

§Implementation Plan step 9:

> Carve out the legacy-code, contracts, and target adapters. Move into their own repositories (or top-level directories in a monorepo). Each ships its own README, manifest, briefs, and WASI tools where applicable. The `surfaces.json` schema and repair loop move with `sources/legacy-code-`*.

`documentation` was correctly removed from the carve-out (it's now in core), but the migration path for the existing `plugins/change/skills/draft/briefs/<target>/analyze.md` prose is no longer named anywhere. The earlier draft's table row said:

> `sources/documentation` … absorbs `plugins/change/skills/draft/briefs/<target>/analyze.md` prose.

That sentence should be added to step 4 or step 11 (the documentation rewrite step), since it's in-core now and the prose has to move *somewhere* — presumably into `sources/documentation/briefs/enumerate.md`.

## Minor polish

### 7. Vocabulary table is missing `default source adapter`

Adding one row stating that `intent` and `documentation` are the two default source adapters shipped in core would make §Vocabulary self-contained against the rest of the document.

### 8. Open Question 1 now covers two adapters

Worth splitting into two questions if either could be answered differently — and worth deciding whether an operator can *uninstall* a default (e.g. a project that only consumes contracts and forbids prose-driven specs might want `documentation` off).

### 9. `Sources:` example uses opaque keys

§Per-requirement provenance and tags shows `Sources: [legacy-monolith, design-doc]`. It would help to add a one-liner stating that `Sources:` lists `sources.yaml` *binding keys*, not source-adapter *names* — and that two slices binding the same docs corpus under different keys would show different provenance lines. Currently the source-key / adapter-name distinction is only inferable from the evidence-pack YAML.

### 10. Acceptance suite could explicitly assert "no `surfaces.json` for non-legacy sources"

§Open Question 3 wants `surfaces.json` to move with the legacy adapter family. A test that asserts a documentation-only slice produces zero `surfaces.json` entries (and a documentation+legacy slice produces one only for the legacy half) makes the carve-out testable rather than trusted.

### 11. `intent` evidence kinds

Implementation Plan step 4 says `intent` extract emits `kind: intent-text` (singular). The closed-kind enum lists ten kinds. Worth saying whether `intent` is allowed to emit `requirement-statement` directly (some operators write specs as crisp REQ-style bullets in their brief), or whether all `intent` evidence is `intent-text` and synthesis derives requirement statements from it. Tiny call but it affects how briefs are written.

## Bottom line

The structural problems posed in the original brief are all addressed by the current revision. The big-ticket item that would block landing is **(1) — synthesis-contract ownership** because it changes what target adapters do, which has reach into RFC-24 and into the whole `targets/<name>/briefs/` family. Resolving that one will probably also clarify (2) and (3), since they're all aspects of "where does the per-source-format intelligence live".
