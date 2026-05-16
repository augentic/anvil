# RFC-20 Survey to Plan

> Status:  Draft - Depends: [RFC-13](archive/rfc-13-extensibility.md), [RFC-23](archive/rfc-23-change-lifecycle.md)

## Abstract

Introduce a mechanical source-survey stage inside `/change:draft` so Specify can turn legacy code into a reviewable migration plan. The legacy input may be one large monolith, many repositories, or a mix of both. v1 stays narrow: scan externally observable surfaces, record the source files those surfaces touch, size the result by production LOC, and emit one reviewable candidate inventory for `propose`.

The goal is not to extract full specs. The goal is to answer one planning question before `propose` runs:

> What are the smallest coherent business capabilities we can migrate, and in what order?

This RFC focusses on the `/change:draft` analysis process. Advanced cross-source pairing, routing inference, detailed detector catalogues, and future reconciliation features are deliberately secondary until real plans show repeated operator pain.

## Motivation

`/change:draft` already knows how to author `plan.yaml` through a brief pipeline: discovery, optional workspace sync, propose, optional assignment, validate, and hand-off. What it does not yet have is a reliable decomposition step for legacy code.

Without that step:

- A 100k LOC monolith reaches planning as one oversized input.
- A fleet of legacy repositories reaches planning as many disconnected inputs.
- Slice boundaries are inferred directly from code organization, which risks rebuilding the legacy architecture in the target system.
- Cross-repo flows such as publisher/subscriber pairs or service-to-service HTTP calls are hard to spot because each repo reaches planning with separate evidence.
- `propose` has to negotiate slice boundaries and plan entries at the same time.

The missing primitive is a source survey: a deterministic analysis pass that turns legacy code into small, slice-sized candidates before `propose` asks the operator to accept, edit, or reject plan entries.

## Core Idea

Legacy code should be decomposed by externally visible behavior, not internal structure.

A surface is an observable entry point or contract edge: an HTTP route, message publication, message subscription, scheduled job, WebSocket handler, UI route, CLI command, or outbound service call. Surfaces are useful because they describe what the system promises to the outside world. Source modules describe how the legacy system happened to implement those promises.

For every legacy source, the survey records:

- The surfaces the source exposes or consumes.
- The handler or call site for each surface.
- The source files reached from that handler or call site.
- Evidence explaining how the surface was found.

Then `/change:survey` reads the mechanical evidence, performs minimal same-source clustering, sizes each candidate, and emits the candidate set consumed by `propose`. It does not try to reconstruct the full legacy architecture, infer cross-source ownership, or route work to target projects in v1.

## `/change:draft` Analysis Flow

After this RFC, the planning pipeline inside `/change:draft` has one extra analysis step:

- **Pre-flight** — confirm the command can run in the current project, validate arguments, and fail early before any plan files are written.
- **Brief scaffold** — create the draft change workspace and deterministic brief structure that later stages append to.
- **Registry validate** — check project and capability registry state so planning uses known targets and declared capabilities.
- **Discovery** — gather planning-level source facts and documentation hints before slice candidates are proposed.
- **[When multi-repo system] Workspace sync** — refresh the mult-repo workspace inventory so repository assignments and target projects reflect the current registry.
- **[When migrating a legacy system] Source survey** — mechanically decompose legacy code into surfaces, code footprints, and slice-sized candidates.
- **Propose** — turn accepted candidates into operator-reviewable plan entries.
- **[When multi-repo system] Assignment** — attach accepted plan entries to the projects or repositories that should own the work.
- **Plan validate** — run the canonical plan validation before handing the draft back to the operator.
- **Hand-off** — stop after producing the reviewed planning artifacts and leave execution to `/change:execute`.

Only the middle of the pipeline changes. The initial scaffold, single-writer rule for `plan.yaml`, final `specify plan validate`, and operator hand-off remain unchanged.

### Step 1: Collect Inputs

`specify change draft` records the change and its inputs. A source may be:

- `legacy-code`: a local path or materialized clone of an application, service, package, or repository.
- `documentation`: architecture notes, API docs, runbooks, or other prose.

The same flow covers one source and many sources. A monolith is simply one `legacy-code` source. A distributed legacy estate is many `legacy-code` sources plus any documentation inputs.

### Step 2: Analyze Documentation Inputs

The discovery brief invokes `/change:analyze` once per `documentation` input. `/change:analyze` extracts planning-level candidate hints into `discovery.md`. Documentation is the only kind `/change:analyze` accepts in v1; legacy code is surveyed end-to-end by `/change:survey` (see [Step 3](#step-3-source-survey-and-decomposition-mechanical)).

Before invoking survey, the discovery brief writes the `## Candidate inventory` heading wrapper into `discovery.md` exactly once. Both `/change:analyze` (for `documentation`) and `/change:survey` (for `legacy-code`) append candidate blocks under that heading; the brief never re-emits it.

### Step 3: Source Survey And Decomposition (mechanical)

For `legacy-code` inputs, `/change:survey` runs the mechanical scanner and the per-source decomposition in one pass. It invokes `specify change survey` (see [Mechanical Scanner](#mechanical-scanner)) once per change — passing every recorded `legacy-code` source as a batch — which writes two sidecars per source-key under the plan working directory:

```text
.specify/plans/<change>/survey/<source-key>/metadata.json
.specify/plans/<change>/survey/<source-key>/surfaces.json
```

`metadata.json` records coarse source facts such as language, LOC, module count, and top-level modules. `surfaces.json` records the source's externally observable surfaces and their code footprints (see [Artifacts](#artifacts)).

This split is the key simplification: plan-time code analysis first produces structural evidence, not slice decisions.

Once the sidecars exist, `/change:survey` walks each source independently. v1 keeps the model shallow: only `source`, `surface`, and `candidate` node kinds exist. There is no intermediate `group` kind, no cross-source pairing pass, and no target-routing inference inside survey.


| Kind        | Sized as                                   |
| ----------- | ------------------------------------------ |
| `source`    | union of surface `touches`                 |
| `surface`   | union of handler or call-site `touches`    |
| `candidate` | dedup union of `touches` within one source |


Only `candidate` leaves are consumed by `propose`. `source` and `surface` nodes are structural and exist to make the candidate inventory reviewable; consumers identify terminals via `kind == "candidate"`.

v1 only descends into each source independently. Matching surfaces across repositories is intentionally deferred; operators can still accept, edit, or combine candidates during `propose`.

For each source, survey applies three decisions:

1. **Size check.** If the source as a whole is `acceptable` (see [Step 5](#step-5-size-and-order-candidates)), emit it as a single terminal candidate covering every surface and stop.
2. **Surface candidates.** Otherwise, treat each surface as the default candidate and size it by its `touches`.
3. **Minimal clustering.** Merge same-source surface candidates only when they share a handler/call site, have heavy `touches` overlap, or are explicitly grouped by documentation in `discovery.md`.

There is no nested descent within a source. If a `too-large` candidate survives minimal clustering without a clean partition, survey marks it `unresolved: true`. Survey exits 0 in that case; `propose` is responsible for refusing to draft a plan entry from an unresolved leaf until the operator resolves it.

### Step 4: Minimal Candidate Clustering

Clustering is deliberately small in v1. Inputs:

- All `surfaces.json` files, processed one source at a time.
- `discovery.md` candidate hints from documentation inputs.

Clustering evidence, in priority order:

1. **Shared `touches` overlap (≥ 50%)** within a source — the scattered-within-source case.
2. **Documentation grouping.** When documentation explicitly groups surfaces under one candidate heading, that grouping is authoritative even if identifiers do not match mechanically.
3. **Shared handler or call site.** Multiple routes, topics, or jobs handled by the same function or class are one candidate when the combined size remains `acceptable`.

Cluster outcomes:

- Surface ids in `surfaces[]` are always namespaced `<source-key>:<surface-id>` so the same identifier from two repos remains distinguishable.
- Surfaces that look related across sources remain separate candidates in v1. The evidence is preserved for operator review, but survey does not merge them or emit dependency edges automatically.
- **`too-large` candidate** that cannot be split further by the signals above → leaf is `unresolved: true`; the operator either edits the candidate during `propose` or rescopes the change.

This keeps v1 focused on the repeatable work: finding externally visible surfaces, measuring their code footprints, and producing a candidate inventory that a human can review.

### Step 5: Size And Order Candidates

Each candidate is sized over **production LOC** (excluding tests, generated code, vendored deps, blank lines, comment-only lines) and falls into one of two buckets:


| Size         | Production LOC | Planning meaning                  |
| ------------ | -------------- | --------------------------------- |
| `acceptable` | `< 1000`       | Slice-sized; emit as candidate.   |
| `too-large`  | `>= 1000`      | Split or mark `unresolved: true`. |


The invariant is simple: `propose` should receive `acceptable` candidates or explicit unresolved items. It should not receive an unsliced monolith or an undifferentiated repo fleet.

Finer-grained buckets (XS/S/M/L/XL) are out of scope for v1 — the only outcomes propose acts on are "emit" and "refuse", and the extra grades are reviewer noise until propose grows behavior that depends on the distinction (see [Out Of Scope](#out-of-scope)).

Ordering is intentionally conservative in v1. Survey does not infer `depends-on` from contract edges; it emits candidates in source order, then surface order, with documentation-derived candidates placed where the existing `propose` flow can review them. Dependency ordering remains a `propose` and operator concern until real plans show a repeated need for mechanical ordering.

### Step 6: Hand Candidates To Propose

Survey appends candidate blocks to the `## Candidate inventory` heading the discovery brief wrote in Step 2. Propose remains the only stage that asks the operator to accept, edit, reject, or abort plan entries. Every accepted entry is still written through `specify plan add`.

Survey produces candidates; propose produces `plan.yaml`.

## Surface Identifier Handling

`surfaces[].identifier` preserves the legacy spelling of the observable surface: route, topic, command, channel, schedule, or outbound call. v1 does not canonicalize identifiers for matching and does not load alias files. The identifier is evidence for review, not an automatic cross-source join key.

Detectors still emit stable `surfaces[].id` values so reruns diff cleanly inside a source. The id only needs to be unique within the source's `surfaces.json`; candidate blocks namespace it as `<source-key>:<surface-id>`.

Cross-source identifier normalization, operator alias files, and capability-owned alias bundles are deferred until real plans repeatedly require mechanical pairing across repositories (see [Out Of Scope](#out-of-scope)).

## Artifacts

### `surfaces.json`

One file per `legacy-code` source. Byte-stable, validated before write.

Conceptual shape:

```json
{
  "version": 1,
  "source-key": "legacy-monolith",
  "language": "typescript",
  "surfaces": [
    {
      "id": "http-post-users",
      "kind": "http-route",
      "identifier": "POST /users",
      "handler": "src/auth/register.ts:registerUser",
      "touches": [
        "src/auth/register.ts",
        "src/notifications/email.ts",
        "src/users/repository.ts"
      ],
      "declared-at": ["src/server.ts:42"]
    }
  ]
}
```

All fields are required. `version` is `1`; bumps go through an RFC update. `surfaces[]` is sorted by `id`; `touches` is sorted alphabetically. No timestamps, no absolute paths, no host-state leaks.

Framework detection is still performed by every detector so applicability gating works (see [Detector Contract](#detector-contract)), but the detected signatures are not persisted on `surfaces.json` in v1 — nothing in survey, propose, or the operator review flow branches on them. The field is reserved for a future revision once a consumer needs it (see [Out Of Scope](#out-of-scope)).

`declared-at` is a flat list of paths (or `path:line` references) where the surface is declared to its framework or runtime — the route mount, publish call site, subscription registration, scheduled-job declaration, command registration, UI route entry, or outbound call site, depending on `kind`. It is the answer to "where in the source code does the detector see the proof that this surface exists?", and is intentionally distinct from `handler` (where the implementation lives) and `touches` (what the implementation reaches).

Entries are sorted alphabetically and are paths relative to `$INPUT_PATH`, optionally `:<line>` suffixed. The list is non-empty: every detected surface must point to at least one declaration site. `/change:survey` exposes a single renderer that emits the field into `survey.md`; the renderer is thin and detector authors never hand-write prose.

A categorical declaration discriminator (e.g. `framework-route`, `pubsub-pairing`, `http-pairing`) is intentionally deferred. v1 has no consumer that branches on declaration category, and future cross-source pairing can add the category it actually needs once that behavior exists. See [Out Of Scope](#out-of-scope) for the re-open trigger.

The surface kind enum is closed in v1:

`http-route`, `message-pub`, `message-sub`, `ws-handler`, `scheduled-job`, `cli-command`, `ui-route`, `external-call-out`.

Unknown kinds fail validation. Extensions require an RFC update so capabilities do not drift into incompatible vocabularies.

### `survey.md`

One file per change. Required sections, in order:

1. `Summary` — source / surface / candidate / unresolved counts.
2. `Source inventory` — one row per input source.
3. `Candidate inventory` — proposed slice-sized leaves and unresolved items.

Each node block is a fenced YAML block following a Markdown sub-heading. Fields appear in fixed order so re-runs diff cleanly:

> `kind`, `sources`, `handler`, `touches`, `surfaces`, `declared-at`, `unresolved`

Omit fields that don't apply to the node's kind. Consumers identify terminal leaves by `kind == "candidate"`.

The fenced-YAML form is the **canonical candidate block shape**. Both `/change:survey` (for `legacy-code` inputs) and `/change:analyze` (for `documentation` inputs) emit blocks in this shape under the shared `## Candidate inventory` heading; propose runs a single parser that keys on field names rather than on the source of the block. Doc-derived blocks omit mechanically-resolved `handler` / `touches` paths when no hint applies; survey-derived blocks always include `kind: candidate`.

Example same-source candidate leaf:

```markdown
### identity.user-registration [acceptable, 894 LOC]

```yaml
kind: candidate
sources: [legacy-monolith]
handler: src/auth/register.ts:registerUser
touches:
  - src/auth/register.ts
  - src/notifications/email.ts
  - src/users/repository.ts
surfaces:
  - legacy-monolith:http-post-users
  - legacy-monolith:message-pub-user-created
declared-at:
  - legacy-monolith:src/server.ts:42
  - legacy-monolith:src/users/events.ts:18
```
```

Example unresolved candidate leaf:

```markdown
### billing.invoice-sync [too-large, 1320 LOC]

```yaml
kind: candidate
sources: [legacy-billing]
touches:
  - src/billing/invoices.ts
  - src/billing/reconciliation.ts
  - src/billing/settlement.ts
surfaces:
  - legacy-billing:scheduled-job-invoice-sync
  - legacy-billing:message-sub-payment-settled
declared-at:
  - legacy-billing:src/billing/scheduler.ts:24
  - legacy-billing:src/billing/subscriptions.ts:11
unresolved: true
```
```

Re-running on unchanged inputs produces byte-identical `survey.md`.

## Mechanical Scanner

The CLI scanner invoked by `/change:survey`. Two forms:

```text
# Single-source form (ad-hoc / debugging)
specify change survey <source-path> --source-key <key> --out <dir>

# Batch form (the form `/change:survey` uses)
specify change survey --sources <file> --out <dir>
```

The verb sits under `specify change` to make its plan-time role explicit and to keep the CLI namespace aligned with the rest of the change-lifecycle surface (`specify change draft`, `specify change finalize`, …).

It owns mechanical work only:

- Detect framework signatures.
- Enumerate surfaces.
- Resolve handlers and call sites where static analysis can do so.
- Record touched files.
- Capture coarse source metadata (language, LOC, module count, top-level modules).
- Validate and write `surfaces.json` and `metadata.json`.

The scanner does not call an LLM, infer candidates, or write `plan.yaml`.

**Flags.**

- `<source-path>` and `--source-key <key>` are the **single-source form** and are mutually exclusive with `--sources`. Both are required when used; ad-hoc invocations must supply the key explicitly so source-key mismatches fail closed.
- `--sources <file>` is the **batch form**. The file is a small YAML document listing one entry per source:

  ```yaml
  version: 1
  sources:
    - key: legacy-monolith
      path: ./legacy/monolith
    - key: legacy-billing
      path: ./legacy/billing
  ```

  `/change:survey` writes this file from the change's recorded `legacy-code` sources, so the whole legacy-code batch reaches the CLI in one invocation. The verb processes each row independently and atomically: a row's `surfaces.json` and `metadata.json` are written iff that row's detectors complete cleanly, and a row failure leaves that row's existing files untouched. Rows that completed cleanly before a later row failed remain on disk so re-runs only re-do the failed work — the per-source-key files are independent.
- `--out <dir>` is a directory. In the single-source form the verb writes `<dir>/surfaces.json` and `<dir>/metadata.json` (the skill is responsible for picking a per-source-key directory). In the batch form `<dir>` is the parent directory and the verb writes `<dir>/<source-key>/surfaces.json` and `<dir>/<source-key>/metadata.json` per row. Either form refuses to overwrite a `surfaces.json` whose `source-key` does not match the requested key.
- `--format` is intentionally absent in v1. The output files are JSON by definition; the flag would re-introduce if and when stdout JSON envelopes are needed for shell pipelines.

**Exit discriminants.** Initial set, kebab-case per the CLI repo's coding standards:

- `no-detectors` — no detector applied to the source.
- `detector-id-collision` — two detectors emitted the same `surfaces[].id`.
- `source-path-missing` — `<source-path>` does not exist (single-source form) or a row's `path` does not exist (batch form).
- `source-path-not-readable` — `<source-path>` cannot be read (single-source form) or a row's `path` cannot be read (batch form).
- `detector-failure` — a detector panicked or returned a malformed `Surface`.
- `sources-file-missing` — the `--sources` file does not exist (batch form).
- `sources-file-malformed` — the `--sources` file is not valid YAML, fails schema validation, or contains a duplicate `key` (batch form).

No partial output is ever written for a row; on any non-zero exit, the affected row's `surfaces.json` and `metadata.json` are left untouched.

Capability-owned detector packages may add framework support over time, but v1 ships a global registry (see [Detector Contract](#detector-contract)). v1 only needs enough detectors to prove the flow on the first supported stack; unsupported stacks fall back to manual source scoping until a detector exists.

## Detector Contract

A detector is a unit of mechanical, framework-specific surface enumeration. The contract is intentionally narrow so the v1 detector layer can be built into the binary without a sandbox.

**Registration.** A thin Rust trait inside `specify-cli`, with a `DetectorRegistry` populated at binary build time. This mirrors the resolver layering in `crates/tool/src/resolver/`* and avoids the overhead of WASI for purely mechanical work. Out-of-tree detector packaging (WASI tool per RFC-13, per-capability detector packs) is **deferred**; revisit when a real out-of-tree capability needs to ship a detector.

**Input shape.**

```rust
struct DetectorInput<'a> {
    source_root: &'a Path,
    language_hint: Option<Language>,
}
```

**Output shape.**

```rust
struct DetectorOutput {
    surfaces: Vec<Surface>,
}
```

`Surface` matches the `surfaces.json` `surfaces[]` entry verbatim (including the `declared-at` field from [Artifacts](#artifacts)). Detectors return owned data; the verb deduplicates, sorts, and writes.

**Discovery rule.** `specify change survey` runs every registered detector against the source root. Each detector self-reports applicability internally: when its framework signatures are absent the detector returns an empty `DetectorOutput { surfaces: vec![] }`. The verb:

1. Merges `surfaces` across all detectors and asserts no two detectors emitted the same `id`; on collision, exits `detector-id-collision`.
2. Validates the merged output against the `surfaces.json` schema and writes it atomically.

**Capability scoping.** v1 is a single global registry. Per-capability detector packs at `plugins/change/skills/survey/briefs/<cap>/detectors/` are explicitly deferred; the directory is reserved but not loaded in v1.

**Failure modes.** A detector that panics or returns a malformed `Surface` fails the run with `detector-failure`; the failing detector's name is included in the error payload so the operator can pin a workaround.

## Skill Responsibility Split


| Component               | Responsibility                                                                                                                                                                                                                                                                                                       |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/change:analyze`       | Extract candidate hints from `documentation` inputs into `discovery.md`. Documentation is the only kind accepted in v1.                                                                                                                                                                                              |
| `specify change survey` | Deterministically enumerate surfaces for one source (single-source form) or a batch of sources (batch form); write `metadata.json` + `surfaces.json` per source-key.                                                                                                                                                 |
| `/change:survey`        | Build the `--sources` batch file from the change's recorded `legacy-code` sources, invoke `specify change survey`, then compose all `surfaces.json` files into one inventory, size candidates, apply minimal same-source clustering, write `survey.md`, and append candidate blocks under the discovery-owned heading. |
| `propose` brief         | Ask the operator to accept/edit/reject candidates and write accepted plan entries through `specify plan add`.                                                                                                                                                                                                        |


This split keeps expensive semantic judgement out of per-source analysis while still giving `propose` one candidate inventory to review.

## Routing Behavior (v1)

Survey-derived candidates do not carry `target-project` in v1. Assignment continues to use today's signals:

1. Description match.
2. Baseline spec affinity.
3. Capability compatibility.
4. Ambiguity → human.

Documentation-derived candidate hints may still carry target routing when the documentation analysis already has explicit evidence, but survey does not propagate that hint onto mechanically scanned leaves. Canonical-owner routing for cross-source leaves is deferred until survey actually emits cross-source leaves.

## Single-Source And Multi-Source Behavior

The algorithm is identical in both cases.

For a monolith, survey usually finds same-source candidates: one source-sized candidate when the source is small enough, or surface-sized candidates with minimal clustering when it is not.

For a repo fleet, survey runs the same source-local algorithm for each input and emits one combined inventory. It may expose evidence that two sources communicate, such as an outbound HTTP call and a matching route, but it does not merge them into one candidate automatically.

The source count changes the breadth of the inventory, not the planning model.

## Brownfield Behavior (v1)

When a target workspace already has `.specify/specs/` baselines, survey treats baseline projects as **opaque routing targets** consumed by existing assignment logic. It does **not** read baselines to flag delta-target opportunities; that is a propose-time concern with no concrete first user. See [Out Of Scope](#out-of-scope).

## Guardrails

- Survey is plan-time decomposition, not spec extraction. Full `spec.md` and `design.md` authoring still happens per slice through `/spec:define`, delegating to `/spec:extract` when legacy code is the source.
- Survey never writes `plan.yaml`. Only `specify change draft`, `specify plan add`, and `specify plan amend` write plan state.
- Legacy module boundaries are evidence, not authority. They may help find surfaces and code footprints, but they do not define slices.
- Unknown surface kinds and malformed sidecars fail closed.
- Outputs are byte-stable on unchanged inputs: fixed field order, sorted lists, no timestamps, no absolute paths, no host-specific state.
- Ambiguity is explicit. Survey emits `unresolved` candidates rather than inventing aliases or silently merging unrelated surfaces.

## Implementation Plan

1. Add the `surfaces.json` schema + validators (`declared-at` non-empty and sorted alphabetically per [Artifacts](#artifacts)).
2. Add `specify change survey` with a stub detector registry, deterministic output, validation before write, the required `--source-key` flag, the `--out <dir>` directory contract, and the initial exit-discriminant set documented in [Mechanical Scanner](#mechanical-scanner).
3. Land the detector trait, `DetectorRegistry`, and `DetectorInput` / `DetectorOutput` shapes per [Detector Contract](#detector-contract), then the first mechanical detectors for the initial supported stack (Express, NestJS, BullMQ). Landing a real detector forces the contract to be exercised end-to-end.
4. Rewrite `/change:analyze` to handle `documentation` inputs only and emit candidate blocks in the unified fenced-YAML shape from [Artifacts](#artifacts). The skill drops its `kind` positional in v1; reintroducing a closed-enum `kind` is a re-open trigger when a new structured-prose kind (e.g. `domain-model`) lands (see [Out Of Scope](#out-of-scope)).
5. **Combined release.** Land the discovery-brief edit that writes the `## Candidate inventory` heading wrapper *together with* the `/change:survey` skill in step 6 — the two must ship in a single PR ("discovery + survey heading handshake") to avoid a half-state where survey expects a heading the brief doesn't write.
6. Add `/change:survey`. The skill builds the `--sources` batch file from the change's recorded `legacy-code` sources, invokes `specify change survey` once, and then performs source-local sizing, surface-sized default candidates, minimal same-source clustering (`touches` overlap, explicit documentation grouping, shared handler/call site), `unresolved: true` markers on `too-large` candidates that cannot be split, and the thin `declared-at` renderer from [Artifacts](#artifacts) (sorted file or `file:line` entries). Wire it between workspace sync and propose.
7. Acceptance fixtures (ship in step 6's PR or immediately after). v1 keeps the proving set small and adds escape-hatch fixtures only when real plans exercise them:
  - Single-source L monolith producing surface-sized candidates with one minimal same-source cluster (core happy path).
  - Multi-source change with **at least two source-keys** producing one combined inventory with separate source-local candidates (proves repo-fleet handling without cross-source pairing).
  - Greenfield documentation-only pass-through (survey skipped entirely).
  - Single-source-already-S no-op (source is its own terminal candidate without further partitioning).
  - `too-large` candidate produced by minimal same-source clustering that cannot be split and is emitted `unresolved: true`.
  - Fresh `/change:draft` end-to-end exercising the discovery brief + `/change:survey` handshake and asserting `## Candidate inventory` is emitted exactly once.

  Escape-hatch fixtures deferred until a real plan exercises them: cross-source pairing, dependency ordering, operator aliases, and alias-resolved `unresolved` round-trips. See [Out Of Scope](#out-of-scope).
8. Tutorials: monolith decomposition and legacy-fleet decomposition, with the legacy-fleet tutorial showing separate source-local candidates and the operator review point where related candidates may be combined. Ship a stub `docs/explanation/legacy-migration-at-scale.md` alongside the tutorials, or defer the full document to the follow-on RFC that owns cross-change scale (RFC-21 / RFC-22 are the natural home).

## Migration

This is a plan-time behavioral change for legacy-code inputs.

**For operators.** `/change:analyze` no longer accepts `legacy-code`; it handles `documentation` inputs only. Legacy code is surveyed end-to-end by `/change:survey`, which builds a `--sources` batch file from the change's recorded `legacy-code` sources, invokes `specify change survey` once to write `metadata.json` + `surfaces.json` per source-key, and then owns source-local candidate clustering and the candidate inventory for propose. In-flight plans do not need conversion — re-running `/change:draft` for a legacy-code change regenerates plan-time scratch artifacts in the new shape. Multi-source changes produce one combined inventory, but related candidates from different sources remain separate until the operator combines or orders them during `propose`.

**For capability authors.** Move the `legacy-code` clustering content out of `plugins/change/skills/draft/briefs/<cap>/analyze.md` into `plugins/change/skills/survey/briefs/<cap>/cluster.md`. `analyze.md` retains only the `documentation` content and updates its emitted candidate block to the unified shape (`kind: candidate` plus the field set described in [Artifacts](#artifacts)); the per-capability `analyze` brief no longer dispatches on a `kind` positional. Surface detectors are registered as in-binary Rust detectors per [Detector Contract](#detector-contract); the `plugins/change/skills/survey/briefs/<cap>/detectors/` directory is reserved but not loaded in v1. Identifier aliases and capability-owned detector packaging are deferred (see [Out Of Scope](#out-of-scope)).

**For skill authors consuming planning artifacts.** New artifacts: `surfaces.json` and `metadata.json` per source under `<plan-dir>/survey/<source-key>/`, and `survey.md` under `<plan-dir>/`. Both schemas pinned, byte-stable. The `## Candidate inventory` heading in `discovery.md` is written exactly once by the discovery brief; both `/change:survey` (for `legacy-code`) and `/change:analyze` (for `documentation`) append candidate blocks under it using the single fenced-YAML grammar defined in [Artifacts](#artifacts). Propose runs a single parser keyed on field names; missing fields default per the SKILL table.

Documentation-only changes skip `/change:survey` entirely. With no `legacy-code` source, the pipeline reaches `propose` directly from discovery — there is nothing to decompose and the survey gate adds ceremony without value.

## Non-Goals

- Extracting full specs from legacy code during draft.
- Replacing the propose accept/edit/reject loop.
- Durable source catalogues or cross-change source caches; those belong to RFC-21.
- A migration ledger or cumulative mapping of migrated surfaces; those belong to RFC-22.
- Brownfield reconciliation against existing `.specify/specs/` baselines.
- LLM fallback detectors for unsupported frameworks.
- A standalone sizing command outside the survey flow.
- Everything in [Out Of Scope](#out-of-scope).

## Out Of Scope

Each item below was considered for v1 and deferred. Re-open triggers are concrete so the bar for adding them back is clear.


| Item                                                                                                     | Re-open when                                                                                                                                         |
| -------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `domain-model` as a third closed-enum kind on `/change:analyze` (structured bounded-context import)      | An operator wants a structured context-map workflow, or documentation analyze repeatedly fails to surface bounded-context attribution routing needs. |
| `synthesize` brief and `## Reconciliation` section in `discovery.md`                                     | Propose repeatedly drafts slices that ignore documented-but-uncoded candidates, or `domain-model` lands and produces a third corpus to reconcile.    |
| `specify plan size` standalone CLI verb                                                                  | Operators report wanting LOC audits outside a draft run (slice review, candidate spot-check).                                                        |
| Per-capability `cut.md` brief separate from `cluster.md`                                                 | A capability author writes a source-local refinement that materially exceeds half a page inside `cluster.md`.                                        |
| Per-capability `sizing.toml` overrides (tighten LOC rubric, add aggregate/endpoint counts)               | A capability demonstrates LOC-only sizing produces persistently wrong slices in operator review.                                                     |
| Cross-source contract pairing (pub/sub, HTTP, WebSocket)                                                | Operators repeatedly combine the same source-local candidates by hand during `propose`, or cross-repo migration plans become unreadable without mechanical pairing. |
| Survey-inferred dependency ordering from contract edges                                                  | Operators repeatedly reorder survey candidates by obvious route/topic dependencies during `propose`.                                                  |
| Survey-emitted `target-project` and canonical-owner routing                                              | Assignment repeatedly misroutes survey candidates and the missing signal can be traced to a mechanical cross-source owner.                            |
| Operator-authored `identifier-aliases.yaml` and per-capability alias bundles                             | Operators repeatedly need alias files to resolve the same cross-source identifiers, or a capability author repeatedly hand-copies the same alias rules. |
| Sub-source `group` DAG nodes and per-source structural depth cap                                         | A source produces enough surfaces that minimal clustering alone yields candidates the operator cannot review without intermediate framework-module structure. |
| Finer-grained sizing buckets (XS/S/M/L/XL or similar)                                                    | Propose grows behavior that branches on more than the `acceptable` / `too-large` distinction (e.g. parallelism hints, review-effort budgeting).      |
| LLM-fallback detector contract and `--fallback-llm` flag                                                 | A real legacy stack outside the mechanical-detector envelope reaches the planning pipeline.                                                          |
| Brownfield reconciliation against `.specify/specs/` baselines (read baselines for delta-target flagging) | Brownfield-only changes reach the pipeline frequently enough that propose's missing delta-target awareness becomes a recurring complaint.            |
| Surface `confidence` field (graded high/medium/low)                                                      | The LLM-fallback contract lands; the field then differentiates mechanical from probabilistic detection.                                              |
| Closed `declaration-kind` discriminator on `declared-at` (e.g. `framework-route`, `pubsub-pairing`, `http-pairing`, …) | A consumer (propose, plan diffing, CI gate, telemetry) needs to branch on declaration category without re-deriving it from `surfaces[]` membership. |
| Persisted `cross-module` / `cross-source` boolean flags on candidate leaves                              | A consumer needs to filter / branch on multi-module or multi-source candidates and the derivation from `sources` + namespaced `surfaces[]` proves expensive or error-prone in practice. |
| Machine-readable JSON sibling for `survey.md`                                                            | A downstream consumer (CI gate, registry sync, telemetry, plan diffing tool) needs structured survey data; v1 stays markdown-only.                   |
| Persisted `framework-signatures` field on `surfaces.json` (detector applicability gating still happens in-process) | A consumer (propose, plan diffing, CI gate, telemetry, capability routing) needs to branch on the detected framework set without re-running detectors. |
| Escape-hatch acceptance fixtures: cross-source pairing, `depends-on` cycle round-trip, alias-resolved `unresolved` on a ≥ 3-source-key plan | A real plan exercises any of these escape hatches and regresses, or the matching code path lands a behavioral change that needs guarding. |


## References

- [RFC-13: Extensibility](archive/rfc-13-extensibility.md)
- [RFC-21: Source Catalogue and Tier-1 Cache](rfc-21-catalogue.md)
- [RFC-22: Migration Ledger](rfc-22-ledger.md)
- [RFC-23: Change Lifecycle](archive/rfc-23-change-lifecycle.md)
- `[/change:draft` SKILL.md](../plugins/change/skills/draft/SKILL.md)
- `[/change:analyze` SKILL.md](../plugins/change/skills/analyze/SKILL.md)

