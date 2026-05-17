# RFC-20 Survey to Plan

> Status: Draft - Depends: [RFC-13](archive/rfc-13-extensibility.md), [RFC-23](archive/rfc-23-change-lifecycle.md)

## Abstract

Introduce a source-survey stage inside `/change:draft` so Specify can turn legacy code into a reviewable migration plan. The legacy input may be one large monolith, many repositories, or a mix of both. v1 stays narrow: enumerate externally observable surfaces with an LLM driven by per-language briefs, validate the output against a closed schema, size the result by production LOC, and emit one reviewable candidate inventory for `propose`.

The goal is not to extract full specs. The goal is to answer one planning question before `propose` runs:

> What are the smallest coherent business capabilities we can migrate, and in what order?

This RFC focusses on the `/change:draft` analysis process. Advanced cross-source pairing, routing inference, mechanical detector reintroduction, and future reconciliation features are deliberately secondary until real plans show repeated operator pain.

## Design History

An earlier draft of RFC-20 specified a mechanical scanner: in-binary Rust detectors per framework (Express, NestJS, BullMQ, …) compiled into `specify-cli`, producing byte-stable `surfaces.json` deterministically. That design held for a single supported stack but did not survive the four-language v1 coverage target (TypeScript, C#, Rust, COBOL):

- Even within TypeScript, the regex-based detector approach already conceded that "function-level call-graph tracing would require an AST parser".
- ASP.NET Core, Axum/Actix/Rocket, and CICS/IMS have no shared idiom that would let a regex detector cover them; each would require its own parser inside the CLI binary.
- Maintenance scaled super-linearly in (frameworks × languages) and crowded out work the planning pipeline actually needs.

This revision pivots the producer of `surfaces.json` from in-binary detectors to per-language LLM enumeration briefs, while keeping the schema, candidate algorithm, sizing rubric, and CLI as the deterministic spine. Pre-1.0 the `Detector` trait and `DetectorRegistry` were deleted as YAGNI carrying cost; the artifact contract (the DTOs and validators in `crates/domain/src/survey/{dto,validate,sources}.rs`) is the reversion seam — a future RFC that reintroduces mechanical enumeration for a (language, framework) pair `git revert`s the trait module rather than carrying it indefinitely.

## Motivation

`/change:draft` already knows how to author `plan.yaml` through a brief pipeline: discovery, optional workspace sync, propose, optional assignment, validate, and hand-off. What it does not yet have is a reliable decomposition step for legacy code.

Without that step:

- A 100k LOC monolith reaches planning as one oversized input.
- A fleet of legacy repositories reaches planning as many disconnected inputs.
- Slice boundaries are inferred directly from code organization, which risks rebuilding the legacy architecture in the target system.
- Cross-repo flows such as publisher/subscriber pairs or service-to-service HTTP calls are hard to spot because each repo reaches planning with separate evidence.
- `propose` has to negotiate slice boundaries and plan entries at the same time.

The missing primitive is a source survey: an analysis pass that turns legacy code into small, slice-sized candidates before `propose` asks the operator to accept, edit, or reject plan entries.

## Core Idea

Legacy code should be decomposed by externally visible behavior, not internal structure.

A surface is an observable entry point or contract edge: an HTTP route, message publication, message subscription, scheduled job, WebSocket handler, UI route, CLI command, or outbound service call. Surfaces are useful because they describe what the system promises to the outside world. Source modules describe how the legacy system happened to implement those promises.

For every legacy source, the survey records:

- The surfaces the source exposes or consumes.
- The handler or call site for each surface.
- The source files reached from that handler or call site.
- The declaration sites that prove the surface exists.

Then `/change:survey` reads the validated evidence, performs minimal same-source clustering, sizes each candidate, and emits the candidate set consumed by `propose`. It does not try to reconstruct the full legacy architecture, infer cross-source ownership, or route work to target projects in v1.

The producer of the per-source `surfaces.json` is an LLM driven by a per-language enumeration brief. The CLI owns validation, metadata capture, and atomic writes — every property the artifact contract requires (closed `kind` enum, sorted lists, paths under the source root, no host-state) is enforced mechanically before any output reaches the rest of the pipeline. Hallucinated surfaces and shape errors fail the run with structured exit discriminants.

## `/change:draft` Analysis Flow

After this RFC, the planning pipeline inside `/change:draft` has one extra analysis step:

- **Pre-flight** — confirm the command can run in the current project, validate arguments, and fail early before any plan files are written.
- **Brief scaffold** — create the draft change workspace and deterministic brief structure that later stages append to.
- **Registry validate** — check project and capability registry state so planning uses known targets and declared capabilities.
- **Discovery** — gather planning-level source facts and documentation hints before slice candidates are proposed.
- **[When multi-repo system] Workspace sync** — refresh the multi-repo workspace inventory so repository assignments and target projects reflect the current registry.
- **[When migrating a legacy system] Source survey** — enumerate surfaces per source via the per-language brief, validate, size, cluster, and produce slice-sized candidates.
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

The discovery brief invokes `/change:analyze` once per `documentation` input. `/change:analyze` extracts planning-level candidate hints into `discovery.md`. Documentation is the only kind `/change:analyze` accepts in v1; legacy code is surveyed end-to-end by `/change:survey` (see [Step 3](#step-3-source-survey-and-decomposition)).

Before invoking survey, the discovery brief writes the `## Candidate inventory` heading wrapper into `discovery.md` exactly once. Both `/change:analyze` (for `documentation`) and `/change:survey` (for `legacy-code`) append candidate blocks under that heading; the brief never re-emits it.

### Step 3: Source Survey And Decomposition

For `legacy-code` inputs, `/change:survey` runs the per-source enumeration brief and the per-source candidate algorithm in one pass. Per source the skill:

1. Resolves the active per-language enumeration brief from the detected `language`.
2. Drives the LLM with that brief over the source root to produce a candidate `surfaces.json`.
3. Hands the candidate to `specify change survey` for validation, metadata capture, canonicalization, and atomic write.
4. Reads the canonicalized output back and runs the candidate algorithm.

The CLI invocation is a batch over every recorded `legacy-code` source — passing one `--sources` file plus a staged-input directory — and writes two sidecars per source-key under the plan working directory:

```text
.specify/plans/<change>/survey/<source-key>/metadata.json
.specify/plans/<change>/survey/<source-key>/surfaces.json
```

`metadata.json` records coarse source facts such as language, LOC, module count, and top-level modules. `surfaces.json` records the source's externally observable surfaces and their code footprints (see [Artifacts](#artifacts)).

This split is the key simplification: the LLM produces structural evidence, not slice decisions. Slice decisions live in the candidate algorithm, which runs deterministically over validated evidence.

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

This keeps v1 focused on the repeatable work: enumerating externally visible surfaces, measuring their code footprints, and producing a candidate inventory that a human can review.

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

The enumeration brief still emits stable `surfaces[].id` values so reruns diff cleanly inside a source. The id only needs to be unique within the source's `surfaces.json`; candidate blocks namespace it as `<source-key>:<surface-id>`.

Cross-source identifier normalization, operator alias files, and capability-owned alias bundles are deferred until real plans repeatedly require pairing across repositories (see [Out Of Scope](#out-of-scope)).

## Determinism Policy

Agent enumeration is non-deterministic by construction. The artifact contract still has to be reproducible enough that re-runs do not churn `survey.md` or `discovery.md`. The policy:

- **Schema-stable.** `surfaces.json` is validated against the closed schema before write. Unknown `kind`, missing required fields, paths outside the source root, absolute paths, or duplicate `id` values fail the run.
- **Sort-stable.** The CLI sorts `surfaces[]` by `id` and sorts `touches[]` and `declared-at[]` alphabetically before writing. The agent's output order does not influence the canonical form.
- **Idempotent on unchanged inputs.** When the agent's surface set, handler resolution, and `touches` set match the prior run, the canonicalized files are byte-identical. Equivalent runs produce equivalent outputs even if the LLM phrasing differed in transit.
- **Pinned per-language brief.** Each enumeration brief lives at a versioned path (`plugins/change/skills/survey/briefs/enumerate/<language>.md`); changes to the brief are reviewable diffs, and re-runs against the same brief produce comparable results.
- **Bounded repair loop.** When the candidate `surfaces.json` fails validation, the skill re-prompts the LLM with the structured error up to a small bounded retry count (v1: three retries). On exhaustion the skill exits with `surveyor-exhausted` and emits the last failing candidate alongside the validator output, so the operator can edit it by hand or re-run with a tighter brief.

This policy is the contract. It softens the byte-identical-on-every-run guarantee from the previous design, but only along an axis the agent controls — the canonical files remain the contract for everything downstream.

## Artifacts

### `surfaces.json`

One file per `legacy-code` source. Validated and canonicalized before write.

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

All fields are required. `version` is `1`; bumps go through an RFC update. `surfaces[]` is sorted by `id`; `touches` is sorted alphabetically. No timestamps, no absolute paths, no host-state leaks. Every path in `touches[]` and `declared-at[]` must resolve to a file under the source root; the CLI validates this before write.

The enumeration brief is encouraged to capture detected framework signatures internally so it can scope its enumeration, but the detected signatures are not persisted on `surfaces.json` in v1 — nothing in survey, propose, or the operator review flow branches on them. The field is reserved for a future revision once a consumer needs it (see [Out Of Scope](#out-of-scope)).

`declared-at` is a flat list of paths (or `path:line` references) where the surface is declared to its framework or runtime — the route mount, publish call site, subscription registration, scheduled-job declaration, command registration, UI route entry, or outbound call site, depending on `kind`. It is the answer to "where in the source code can the operator see proof that this surface exists?", and is intentionally distinct from `handler` (where the implementation lives) and `touches` (what the implementation reaches).

Entries are sorted alphabetically and are paths relative to the source root, optionally `:<line>` suffixed. The list is non-empty: every emitted surface must point to at least one declaration site. `/change:survey` exposes a single renderer that emits the field into `survey.md`; the renderer is thin and the brief never hand-writes prose.

A categorical declaration discriminator (e.g. `framework-route`, `pubsub-pairing`, `http-pairing`) is intentionally deferred. v1 has no consumer that branches on declaration category, and future cross-source pairing can add the category it actually needs once that behavior exists. See [Out Of Scope](#out-of-scope) for the re-open trigger.

The surface kind enum is closed in v1:

`http-route`, `message-pub`, `message-sub`, `ws-handler`, `scheduled-job`, `cli-command`, `ui-route`, `external-call-out`.

Unknown kinds fail validation. Extensions require an RFC update so capabilities and language briefs do not drift into incompatible vocabularies.

### `survey.md`

One file per change. Required sections, in order:

1. `Summary` — source / surface / candidate / unresolved counts.
2. `Source inventory` — one row per input source.
3. `Candidate inventory` — proposed slice-sized leaves and unresolved items.

Each node block is a fenced YAML block following a Markdown sub-heading. Fields appear in fixed order so re-runs diff cleanly:

> `kind`, `sources`, `handler`, `touches`, `surfaces`, `declared-at`, `unresolved`

Omit fields that don't apply to the node's kind. Consumers identify terminal leaves by `kind == "candidate"`.

The fenced-YAML form is the **canonical candidate block shape**. Both `/change:survey` (for `legacy-code` inputs) and `/change:analyze` (for `documentation` inputs) emit blocks in this shape under the shared `## Candidate inventory` heading; propose runs a single parser that keys on field names rather than on the source of the block. Doc-derived blocks omit handler / touches paths when no hint applies; survey-derived blocks always include `kind: candidate`.

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

Re-running on unchanged inputs produces `survey.md` with the same candidate set, sizes, and field values — modulo the determinism policy above. Narrative phrasing inside `Summary` is generated from the validated counts and is therefore stable too.

## Agent Enumeration

`/change:survey` enumerates surfaces by driving an LLM against a per-language brief. The brief lives at `plugins/change/skills/survey/briefs/enumerate/<language>.md` and is the only place per-language knowledge ships.

**Brief contents.** Each brief carries:

- The language and target frameworks it covers.
- A worked example (input snippet → expected `Surface` block) for every kind in the closed enum that applies to the language.
- Anti-patterns the brief should never emit (e.g. dead code, unreachable handlers, type-only files in `touches[]`).
- Constraints on `handler` and `touches[]` resolution: relative paths under the source root only, no `..` traversal, no `node_modules` / `vendor` / `target` / `.venv`.
- The schema's closed-enum kinds and required-field list, repeated verbatim.

**v1 brief set.**

- TypeScript / JavaScript (Express, NestJS, BullMQ, Fastify, Next.js API routes).
- C# (.NET 6+ controllers, minimal API, MassTransit, MediatR, Hangfire).
- Rust (Axum, Actix, Rocket, common pub/sub crates).
- COBOL (CICS BMS, IMS DC, MQ Series, batch JCL job steps).

Stacks outside the brief set fall back to manual source scoping until a brief exists. Adding a new brief is a documentation-only change inside the `specify` repo; it does not require a CLI release.

**Producer contract.** The brief produces a JSON document matching the `surfaces.json` schema verbatim. The skill writes the candidate to a staging path and hands it to `specify change survey`, which validates schema + invariants and either canonicalizes it into the canonical output location or fails with a structured error the skill feeds back into the repair loop.

**No LLM in the validator.** The CLI never calls an LLM. Validation, sorting, metadata capture, atomic writes, and exit-code mapping are deterministic.

**Future mechanical reversion.** Pre-1.0 the `Detector` trait, `DetectorRegistry`, and `merge_detector_outputs` helper were deleted as YAGNI carrying cost — empty registries, an unreachable merge helper, and a `Language` enum with zero production callers had become 215 LOC of dead surface plus 275 LOC of tests exercising nothing the binary could reach. v1 routes every legacy-code source through the agent path. A future RFC that reintroduces mechanical enumeration for a specific (language, framework) pair `git revert`s those modules and wires the resulting detector into the survey verb; the artifact contract — the DTOs and validators in `crates/domain/src/survey/{dto,validate,sources}.rs` — does not change in either direction. See [Out Of Scope](#out-of-scope).

## CLI Verb

The `/change:survey` skill calls one CLI verb. Two forms:

```text
# Single-source form (ad-hoc / debugging)
specify change survey <source-path> --source-key <key> --surfaces <input.json> --out <dir>

# Batch form (the form `/change:survey` uses)
specify change survey --sources <file> --staged <dir> --out <dir>
```

The verb sits under `specify change` to make its plan-time role explicit and to keep the CLI namespace aligned with the rest of the change-lifecycle surface (`specify change draft`, `specify change finalize`, …).

It owns deterministic work only:

- Read the candidate `surfaces.json` produced by the skill from `--surfaces` (single-source) or `<staged-dir>/<source-key>.json` (batch).
- Validate against the `surfaces.json` schema and the cross-row invariants (closed `kind` enum, sorted lists, paths under source root, no duplicate `id`).
- Capture coarse source metadata (language, LOC, module count, top-level modules) from the source path.
- Canonicalize the validated `surfaces.json` (sort `surfaces[]` by `id`; sort `touches[]` and `declared-at[]` alphabetically).
- Write `surfaces.json` and `metadata.json` atomically per source-key.

The verb does not call an LLM, infer candidates, or write `plan.yaml`.

A `--validate-only` flag short-circuits the metadata-and-write step. The skill's repair loop uses it to surface validator errors to the LLM without touching the canonical output directory.

**`--sources` file.** Small YAML document listing one entry per source:

```yaml
version: 1
sources:
  - key: legacy-monolith
    path: ./legacy/monolith
  - key: legacy-billing
    path: ./legacy/billing
```

`/change:survey` writes this file from the change's recorded `legacy-code` sources, so the whole legacy-code batch reaches the CLI in one invocation. The verb processes each row independently and atomically: a row's `surfaces.json` and `metadata.json` are written iff that row's candidate validates, and a row failure leaves that row's existing files untouched. Rows that completed cleanly before a later row failed remain on disk so re-runs only re-do the failed work.

**Staged directory layout (batch form).** For each `<source-key>` listed in `--sources`, the skill writes the candidate to `<staged-dir>/<source-key>.json` before invoking the verb. Missing staged inputs fail with `staged-input-missing`.

**`--out`.** A directory. In the single-source form the verb writes `<dir>/surfaces.json` and `<dir>/metadata.json`. In the batch form `<dir>` is the parent directory and the verb writes `<dir>/<source-key>/surfaces.json` and `<dir>/<source-key>/metadata.json` per row. Either form refuses to overwrite a `surfaces.json` whose `source-key` does not match the requested key.

**Exit discriminants.** Initial set, kebab-case per the CLI repo's coding standards:

- `staged-input-missing` — the candidate `surfaces.json` for a row does not exist.
- `staged-input-malformed` — the candidate is not valid JSON.
- `surfaces-validation-failed` — the candidate fails schema or invariant validation. Detail includes the first failing rule and the offending field path.
- `surfaces-id-collision` — the candidate contains two surfaces with the same `id`.
- `surfaces-touches-out-of-tree` — a `touches[]` or `declared-at[]` entry resolves outside the source root.
- `source-path-missing` — `<source-path>` does not exist (single-source) or a row's `path` does not exist (batch).
- `source-path-not-readable` — `<source-path>` cannot be read.
- `source-key-mismatch` — an existing canonical `surfaces.json` has a different `source-key` than the row requests.
- `sources-file-missing` — the `--sources` file does not exist (batch).
- `sources-file-malformed` — the `--sources` file is not valid YAML, fails schema validation, or contains a duplicate `key` (batch).

No partial output is ever written for a row; on any non-zero exit, the affected row's `surfaces.json` and `metadata.json` are left untouched.

## Skill Responsibility Split

| Component               | Responsibility                                                                                                                                                                                                                                                                                                                                                       |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/change:analyze`       | Extract candidate hints from `documentation` inputs into `discovery.md`. Documentation is the only kind accepted in v1.                                                                                                                                                                                                                                              |
| `/change:survey`        | For each `legacy-code` source: detect language, drive the per-language enumeration brief to produce a candidate `surfaces.json`, hand it to the CLI for validation, run the bounded repair loop on validation failure, then size candidates, apply minimal same-source clustering, write `survey.md`, and append candidate blocks under the discovery-owned heading. |
| `specify change survey` | Validate the candidate `surfaces.json`, enforce closed-enum and path invariants, capture metadata, canonicalize, write atomically per source-key. JSON-only; no markdown, no LLM.                                                                                                                                                                                    |
| `propose` brief         | Ask the operator to accept/edit/reject candidates and write accepted plan entries through `specify plan add`.                                                                                                                                                                                                                                                        |

This split keeps semantic judgement out of the validator and out of `propose`, while keeping the artifact contract enforced by code rather than by prompt.

## Routing Behavior (v1)

Survey-derived candidates do not carry `target-project` in v1. Assignment continues to use today's signals:

1. Description match.
2. Baseline spec affinity.
3. Capability compatibility.
4. Ambiguity → human.

Documentation-derived candidate hints may still carry target routing when the documentation analysis already has explicit evidence, but survey does not propagate that hint onto enumerated leaves. Canonical-owner routing for cross-source leaves is deferred until survey actually emits cross-source leaves.

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
- Unknown surface kinds, paths outside the source root, and malformed sidecars fail closed.
- The CLI validator is mechanical. The closed `kind` enum, sort order, path-under-root rule, and `id` uniqueness check are enforced by code, not by prompt.
- Outputs are schema-stable and sort-stable: fixed field order, sorted lists, no timestamps, no absolute paths, no host-specific state. Equivalent runs produce equivalent canonical files.
- Ambiguity is explicit. Survey emits `unresolved` candidates rather than inventing aliases or silently merging unrelated surfaces.

## Implementation Plan

The detailed plan with phases, change boundaries, and dependencies lives in [`rfc-20-plan.md`](rfc-20-plan.md). High-level sequence:

1. Retire the in-tree mechanical detectors (Express, NestJS, BullMQ), their fixtures, and the now-unreachable `Detector` trait + `DetectorRegistry` + `merge_detector_outputs` modules from `specify-cli`. Keep the schemas, DTOs, validators, and the `--sources` file format as the artifact-contract spine.
2. Refactor `specify change survey` from "run detectors → write" to "ingest staged candidate → validate → canonicalize → capture metadata → write". Introduce the new exit-discriminant set, the `--validate-only` flag, and the `--surfaces` / `--staged` inputs.
3. Add the per-language enumeration briefs (`typescript.md`, `csharp.md`, `rust.md`, `cobol.md`) under `plugins/change/skills/survey/briefs/enumerate/`.
4. Update `/change:survey` SKILL.md and references to drive the per-language brief, run the bounded repair loop, and call the new CLI shape.
5. Refresh the survey fixtures so each fixture's `inputs/` includes a staged candidate `surfaces.json` and the test exercises the validation + canonicalization path. Drop fixtures that exercised the retired in-binary detectors.
6. Update the discovery-handshake and end-to-end fixtures, the monolith / legacy-fleet tutorials, and the project rules / AGENTS.md skill family table.

## Migration

This is a plan-time behavioral change for legacy-code inputs.

**For operators.** `/change:analyze` still handles `documentation` inputs only. Legacy code is surveyed end-to-end by `/change:survey`, which now drives a per-language LLM brief to enumerate surfaces, then hands them to `specify change survey` for validation and canonicalization. Operators who used the previous in-binary detectors get broader language coverage (TypeScript, C#, Rust, COBOL) at the cost of agent latency on plan-time runs. In-flight plans do not need conversion — re-running `/change:draft` for a legacy-code change regenerates plan-time scratch artifacts in the new shape.

**For capability authors.** The capability-axis clustering briefs at `plugins/change/skills/survey/briefs/<cap>/cluster.md` remain unchanged. Per-language enumeration briefs live on a separate axis under `briefs/enumerate/<language>.md` and are owned globally; capabilities are not expected to ship per-language enumeration overrides in v1.

**For skill authors consuming planning artifacts.** Artifact paths and shapes are unchanged: `surfaces.json` and `metadata.json` per source under `<plan-dir>/survey/<source-key>/`, and `survey.md` under `<plan-dir>/`. Both schemas pinned. The `## Candidate inventory` heading in `discovery.md` is written exactly once by the discovery brief; both `/change:survey` (for `legacy-code`) and `/change:analyze` (for `documentation`) append candidate blocks under it using the single fenced-YAML grammar defined in [Artifacts](#artifacts).

Documentation-only changes skip `/change:survey` entirely. With no `legacy-code` source, the pipeline reaches `propose` directly from discovery.

**For the CLI.** `cargo make ci` must regenerate man pages after the verb's flags change. The retirement of `crates/domain/src/survey/detectors/` and `crates/domain/tests/fixtures/detectors/` is part of the same release; no consumer outside this RFC depends on those modules.

## Non-Goals

- Extracting full specs from legacy code during draft.
- Replacing the propose accept/edit/reject loop.
- Durable source catalogues or cross-change source caches; those belong to RFC-21.
- A migration ledger or cumulative mapping of migrated surfaces; those belong to RFC-22.
- Brownfield reconciliation against existing `.specify/specs/` baselines.
- A standalone sizing command outside the survey flow.
- Everything in [Out Of Scope](#out-of-scope).

## Out Of Scope

Each item below was considered for v1 and deferred. Re-open triggers are concrete so the bar for adding them back is clear.

| Item                                                                                                                                        | Re-open when                                                                                                                                                             |
| ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| In-binary mechanical detectors for a (language, framework) pair                                                                             | The agent enumeration brief for that pair shows persistent shape errors, latency complaints, or cost overruns the bounded repair loop cannot absorb.                     |
| Capability-owned per-language enumeration briefs                                                                                            | A capability author needs an enumeration brief that materially differs from the global one (e.g. an Omnia-only runtime adds a new surface kind).                         |
| `domain-model` as a third closed-enum kind on `/change:analyze` (structured bounded-context import)                                         | An operator wants a structured context-map workflow, or documentation analyze repeatedly fails to surface bounded-context attribution routing needs.                     |
| `synthesize` brief and `## Reconciliation` section in `discovery.md`                                                                        | Propose repeatedly drafts slices that ignore documented-but-uncoded candidates, or `domain-model` lands and produces a third corpus to reconcile.                        |
| `specify plan size` standalone CLI verb                                                                                                     | Operators report wanting LOC audits outside a draft run (slice review, candidate spot-check).                                                                            |
| Per-capability `cut.md` brief separate from `cluster.md`                                                                                    | A capability author writes a source-local refinement that materially exceeds half a page inside `cluster.md`.                                                            |
| Per-capability `sizing.toml` overrides (tighten LOC rubric, add aggregate/endpoint counts)                                                  | A capability demonstrates LOC-only sizing produces persistently wrong slices in operator review.                                                                         |
| Cross-source contract pairing (pub/sub, HTTP, WebSocket)                                                                                    | Operators repeatedly combine the same source-local candidates by hand during `propose`, or cross-repo migration plans become unreadable without mechanical pairing.      |
| Survey-inferred dependency ordering from contract edges                                                                                     | Operators repeatedly reorder survey candidates by obvious route/topic dependencies during `propose`.                                                                     |
| Survey-emitted `target-project` and canonical-owner routing                                                                                 | Assignment repeatedly misroutes survey candidates and the missing signal can be traced to a mechanical cross-source owner.                                               |
| Operator-authored `identifier-aliases.yaml` and per-capability alias bundles                                                                | Operators repeatedly need alias files to resolve the same cross-source identifiers, or a capability author repeatedly hand-copies the same alias rules.                  |
| Sub-source `group` DAG nodes and per-source structural depth cap                                                                            | A source produces enough surfaces that minimal clustering alone yields candidates the operator cannot review without intermediate framework-module structure.            |
| Finer-grained sizing buckets (XS/S/M/L/XL or similar)                                                                                       | Propose grows behavior that branches on more than the `acceptable` / `too-large` distinction (e.g. parallelism hints, review-effort budgeting).                          |
| Brownfield reconciliation against `.specify/specs/` baselines (read baselines for delta-target flagging)                                    | Brownfield-only changes reach the pipeline frequently enough that propose's missing delta-target awareness becomes a recurring complaint.                                |
| Surface `confidence` field (graded high/medium/low)                                                                                         | A consumer needs to differentiate enumeration confidence levels (e.g. distinguish brief-covered surfaces from operator-edited ones).                                     |
| Closed `declaration-kind` discriminator on `declared-at`                                                                                    | A consumer (propose, plan diffing, CI gate, telemetry) needs to branch on declaration category without re-deriving it from `surfaces[]` membership.                      |
| Persisted `cross-module` / `cross-source` boolean flags on candidate leaves                                                                 | A consumer needs to filter / branch on multi-module or multi-source candidates and the derivation from `sources` + namespaced `surfaces[]` proves expensive in practice. |
| Machine-readable JSON sibling for `survey.md`                                                                                               | A downstream consumer (CI gate, registry sync, telemetry, plan diffing tool) needs structured survey data; v1 stays markdown-only.                                       |
| Persisted `framework-signatures` field on `surfaces.json`                                                                                   | A consumer (propose, plan diffing, CI gate, telemetry, capability routing) needs to branch on the detected framework set without re-running enumeration.                 |
| Escape-hatch acceptance fixtures: cross-source pairing, `depends-on` cycle round-trip, alias-resolved `unresolved` on a ≥ 3-source-key plan | A real plan exercises any of these escape hatches and regresses, or the matching code path lands a behavioral change that needs guarding.                                |

## References

- [RFC-13: Extensibility](archive/rfc-13-extensibility.md)
- [RFC-21: Source Catalogue and Tier-1 Cache](rfc-21-catalogue.md)
- [RFC-22: Migration Ledger](rfc-22-ledger.md)
- [RFC-23: Change Lifecycle](archive/rfc-23-change-lifecycle.md)
- [`/change:draft` SKILL.md](../plugins/change/skills/draft/SKILL.md)
- [`/change:survey` SKILL.md](../plugins/change/skills/survey/SKILL.md)
- [`/change:analyze` SKILL.md](../plugins/change/skills/analyze/SKILL.md)