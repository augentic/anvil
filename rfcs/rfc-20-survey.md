# RFC-20 Survey to Plan

> Status:  Draft - Depends: [RFC-13](archive/rfc-13-extensibility.md), [RFC-23](archive/rfc-23-change-lifecycle.md)

## Abstract

Introduce a mechanical source-survey stage inside `/change:draft` so Specify can turn legacy code into a reviewable migration plan. The legacy input may be one large monolith, many repositories, or a mix of both. The survey decomposes code from the outside in: externally observable surfaces first, the source files those surfaces touch second, slice-sized units of business capability last.

The goal is not to extract full specs. The goal is to answer one planning question before `propose` runs:

> What are the smallest coherent business capabilities we can migrate, and in what order?

This RFC focusses on the `/change:draft` analysis process. Detailed schemas, detector catalogues, and future reconciliation features are deliberately secondary.

## Motivation

`/change:draft` already knows how to author `plan.yaml` through a brief pipeline: discovery, optional workspace sync, propose, optional assignment, validate, and hand-off. What it does not yet have is a reliable decomposition step for legacy code.

Without that step:

- A 100k LOC monolith reaches planning as one oversized input.
- A fleet of legacy repositories reaches planning as many disconnected inputs.
- Slice boundaries are inferred directly from code organization, which risks rebuilding the legacy architecture in the target system.
- Cross-repo flows such as publisher/subscriber pairs or service-to-service HTTP calls must be stitched together by hand.
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

Then `/change:survey` composes all sources together, clusters related surfaces into business capabilities — or migration candidates, sizes each candidate, and emits the ordered candidate set consumed by `propose`.

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

### Step 2: Analyze Each Input

The discovery brief still invokes `/change:analyze` once per input.

For `documentation`, `/change:analyze` behaves as it does today: it extracts planning-level candidate hints into `discovery.md`.

For `legacy-code`, `/change:analyze` becomes mechanical. It does not infer candidate summaries directly. Instead, it invokes `specify change survey` and writes two sidecars under the plan working directory:

```text
.specify/plans/<change>/analyze/<source-key>/metadata.json
.specify/plans/<change>/analyze/<source-key>/surfaces.json
```

`metadata.json` records coarse source facts such as language, LOC, module count, and top-level modules. `surfaces.json` records the source's externally observable surfaces and their code footprints (see [Artifacts](#artifacts)).

This split is the key simplification: plan-time code analysis first produces structural evidence, not slice decisions.

Before invoking survey, the discovery brief writes the `## Candidate inventory` heading wrapper into `discovery.md` exactly once. Survey appends candidate blocks under that heading; the brief never re-emits it.

### Step 3: Pass 1 — Source Decomposition (mechanical, top-down)

After all inputs have been analyzed, `/change:survey` walks each source independently. v1 keeps the DAG shallow: only `source`, `surface`, and `candidate` node kinds exist. There is no intermediate `group` kind — framework module boundaries, URI/topic prefixes, and worker-pool affinity surface as Pass 2 clustering signals rather than as their own DAG nodes.


| Kind        | Sized as                                              |
| ----------- | ----------------------------------------------------- |
| `source`    | union of surface `touches`                            |
| `surface`   | union of handler `touches`                            |
| `candidate` | dedup union of `touches` across participating sources |


Only `candidate` leaves are consumed by `propose`. `source` and `surface` nodes are structural and exist to make the DAG reviewable; consumers identify terminals via `kind == "candidate"`.

Pass 1 only descends into each source independently. There is no Pass 1 cross-source decomposition; cross-source pairing happens in Pass 2 against normalized identifiers, never against source code.

For each source, Pass 1 applies a single decision:

1. **Size check.** If the source as a whole is `acceptable` (see [Step 5](#step-5-size-and-order-candidates)), emit it as a single terminal candidate covering every surface and stop.
2. **Otherwise, enumerate surfaces.** Hand the full surface set to Pass 2 for clustering.

There is no nested descent within a source. The previous structural depth cap is unnecessary once the DAG bottoms out at the surface level in one step; if a `too_large` cluster survives Pass 2 without a clean partition, Pass 2 marks it `unresolved: true` (see [Step 4](#step-4-pass-2--candidate-clustering-semantic-bottom-up)). Survey exits 0 in that case; `propose` is responsible for refusing to draft a plan entry from an unresolved leaf until the operator resolves it.

### Step 4: Pass 2 — Candidate Clustering (semantic, bottom-up)

Once Pass 1 ends at surfaces, cluster surfaces into candidate leaves. Inputs:

- All `surfaces.json` files, intra-source first.
- `discovery.md` candidate hints from documentation inputs.
- `<plan-dir>/identifier-aliases.yaml` (operator overrides on top of framework defaults; see [Identifier Normalization](#identifier-normalization)).

Clustering evidence, in priority order:

1. **Shared `touches` overlap (≥ 50%)** within a source — the scattered-within-source case.
2. **Documentation grouping.** When documentation explicitly groups surfaces under one candidate heading, that grouping is authoritative even if identifiers do not match mechanically.
3. **Cross-source contract edges**, matched on normalized `identifier`:
  - **Pub/sub pairing.** `message-pub` in source A + `message-sub` in source B sharing the normalized identifier → one cross-source leaf. **Publisher's source is canonical owner**; subscribers join.
  - **HTTP contract pairing.** `external-call-out` in source A whose normalized identifier matches an `http-route` in source B → one cross-source leaf. **Route owner is canonical**; caller depends on it.
  - **WebSocket contract pairing.** `external-call-out` (channel kind) matching a `ws-handler` → one cross-source leaf. **Handler owner is canonical**.
4. **Framework module boundary.** Surfaces whose handlers cluster inside the same `@Module`, Rails engine, Spring `@Configuration`, Phoenix context, etc., when the partition is clean.
5. **URI / topic / channel prefix.** Surfaces sharing a longest-common identifier prefix (`/users/`*, `user.`*) when distinct prefixes have low `touches` overlap.
6. **Worker-pool / scheduled-job batch.** Workers and jobs sharing a topic or schedule prefix.

Cluster outcomes:

- Surface ids in `surfaces[]` are always namespaced `<source-key>:<surface-id>` so the same identifier from two repos remains distinguishable. Whether a candidate is single-source, multi-module within a source, or multi-source is observable directly from `sources` and the namespaced `surfaces[]` — no derived `cross_*` flags are persisted on the candidate (see [Out Of Scope](#out-of-scope) for the re-open trigger).
- `**depends_on` / `depends_on_by`** derive from contract edges (canonical owner → consumer). When producer and consumer end up in the same leaf, no edge is emitted — the dependency is internal.
- **Subscriber surface with no in-scope publisher** → record as a `consumes-external` annotation on its single-source leaf, not an `unresolved`.
- **Ambiguous match** (multiple plausible cross-source pairings after normalization, or an alias-resolved pair the operator has not confirmed) → leaf is `unresolved: true` with the candidate set listed verbatim. Survey never invents fictitious cross-source pairs.
- **`too_large` cluster** that cannot be split further by the signals above → leaf is `unresolved: true`; the operator either extends `identifier-aliases.yaml` to separate the cluster or rescopes the change.

Pass 2 has no depth cap; it is a single pass over the surface set.

### Step 5: Size And Order Candidates

Each candidate is sized over **production LOC** (excluding tests, generated code, vendored deps, blank lines, comment-only lines) and falls into one of two buckets:


| Size         | Production LOC | Planning meaning                  |
| ------------ | -------------- | --------------------------------- |
| `acceptable` | `< 1000`       | Slice-sized; emit as candidate.   |
| `too_large`  | `>= 1000`      | Split or mark `unresolved: true`. |


For a cross-source candidate, LOC is the **deduplicated union of `touches` across every participating source**.

The invariant is simple: `propose` should receive `acceptable` candidates or explicit unresolved items. It should not receive an unsliced monolith or an undifferentiated repo fleet.

Finer-grained buckets (XS/S/M/L/XL) are out of scope for v1 — the only outcomes propose acts on are "emit" and "refuse", and the extra grades are reviewer noise until propose grows behavior that depends on the distinction (see [Out Of Scope](#out-of-scope)).

Ordering comes from `depends_on`. Independent candidates may appear at the same order level and migrate in parallel.

When `depends_on` forms a cycle, mark every participating leaf `unresolved: true` with `cycle_with: [<leaf-ids>]` in the unresolved candidate set and omit the cycle members from the migration order. The operator either breaks the cycle by editing `identifier-aliases.yaml` (collapsing pairs into one leaf) or by re-scoping the change.

### Step 6: Hand Candidates To Propose

Survey appends candidate blocks to the `## Candidate inventory` heading the discovery brief wrote in Step 2. Propose remains the only stage that asks the operator to accept, edit, reject, or abort plan entries. Every accepted entry is still written through `specify plan add`.

Survey produces candidates; propose produces `plan.yaml`.

## Identifier Normalization

Cross-source matching keys on a canonicalised form of `surfaces[].identifier`. Original identifiers are preserved verbatim on every surface; the normalized form is only the matching key.

Framework defaults — explicit, identical for every capability:


| Surface kind                                        | Default canonicalisation                                                                                                                                                                                                                                           |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `http-route`, `ui-route`                            | Lowercase host/path, strip trailing slash, fold path-parameter syntax (`{id}` ≡ `:id` ≡ `<id>`). Strip configured version prefixes (`/v1`, `/v2`, …) by default; opt out per change with `http: { strip_version_prefix: false }` to keep `/v1` and `/v2` distinct. |
| `message-pub`, `message-sub`, `ws-handler`          | Case-fold, unify dot/dash/underscore separators (`user.created` ≡ `user-created` ≡ `user_created`). Strip configured environment prefixes (`prod.`, `staging.`) **only when** listed.                                                                              |
| `cli-command`, `scheduled-job`, `external-call-out` | Lowercase identifier; otherwise verbatim.                                                                                                                                                                                                                          |


After framework canonicalisation, the operator may override matches in `<plan-dir>/identifier-aliases.yaml`. v1 ships one alias tier — operator overrides on top of framework defaults — and no capability-owned alias bundles. Capability-level alias bundles are deferred (see [Out Of Scope](#out-of-scope)) until a capability author demonstrates the same alias rules being hand-copied across multiple plan-dir alias files.

Alias schema:

```yaml
aliases:
  - kind: message-pub
    group: [user.created, users.created, user-created]
http:
  strip_version_prefix: false  # default true; set false to keep /v1 vs /v2 distinct
```

Aliases inside a `group` are bidirectional. Any alias whose `kind` fails the closed `surface kind` enum check **fails the survey**.

Aliases are a review mechanism, not a guess. Survey marks ambiguous matches `unresolved` until the operator confirms the equivalence.

## Artifacts

### `surfaces.json`

One file per `legacy-code` source. Byte-stable, validated before write.

Conceptual shape:

```json
{
  "version": 1,
  "source_key": "legacy-monolith",
  "language": "typescript",
  "framework_signatures": ["express", "bullmq"],
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
      "evidence": {
        "citations": ["src/server.ts:42", "src/auth/register.ts"],
        "note": "express"
      }
    }
  ]
}
```

All fields are required. `version` is `1`; bumps go through an RFC update. `surfaces[]` is sorted by `id`; `touches` is sorted alphabetically; `framework_signatures` is sorted alphabetically. No timestamps, no absolute paths, no host-state leaks.

`evidence` is a small structured shape rather than free prose so byte-stability survives detector phrasing changes:


| Field       | Type     | Notes                                                                                  |
| ----------- | -------- | -------------------------------------------------------------------------------------- |
| `citations` | string[] | Sorted alphabetically; paths relative to `$INPUT_PATH`, optionally `:<line>` suffixed. |
| `note`      | string?  | Optional; kebab-case identifier (typically the framework name, topic, or route slug); no free prose. |


Byte-stability is mechanical: sorted citations + a constrained note field. `/change:survey` exposes a single renderer that emits the shape into `survey.md`, but the renderer is now thin — detector authors never hand-write evidence prose, and consumers can read the structured form directly without a discriminator.

A categorical `kind` discriminator (e.g. `framework-route`, `pubsub-pairing`, `http-pairing`) is intentionally deferred. The pairing type of any candidate is derivable from its `surfaces[]` (a candidate that contains both `message-pub` and `message-sub` of the same normalized identifier is a pub/sub pairing); locking a closed enum now would commit v1 to seven detector-side categories before any of them has a concrete consumer. See [Out Of Scope](#out-of-scope) for the re-open trigger.

The surface kind enum is closed in v1:

`http-route`, `message-pub`, `message-sub`, `ws-handler`, `scheduled-job`, `cli-command`, `ui-route`, `external-call-out`.

Unknown kinds fail validation. Extensions require an RFC update so capabilities do not drift into incompatible vocabularies.

### `survey.md`

One file per change. Required sections, in order:

1. `Summary` — source / surface / candidate / unresolved counts.
2. `Source inventory` — one row per input source.
3. `DAG` — source → surface → candidate (one tree per source).
4. `Candidates` — proposed slice-sized leaves.
5. `Unresolved` — ambiguous or oversized items requiring operator input.
6. `Migration order` — topological sort over `depends_on`.

Each node block is a fenced YAML block following a Markdown sub-heading. Fields appear in fixed order so re-runs diff cleanly:

> `kind`, `sources`, `target_project`, `handler`, `touches`, `surfaces`, `evidence`, `children`, `depends_on`, `depends_on_by`, `unresolved`, `cycle_with`

Omit fields that don't apply to the node's kind. Consumers identify terminal leaves by `kind == "candidate"`. `cycle_with` is emitted only on `unresolved: true` leaves that participate in a `depends_on` cycle.

The fenced-YAML form is the **canonical candidate block shape**. Both survey and `/change:analyze documentation` emit blocks in this shape under the shared `## Candidate inventory` heading; propose runs a single parser that keys on field names rather than on the source of the block. Doc-derived blocks omit the survey-only fields (`target_project` when no hint applies, `cycle_with` on cycle members, mechanically-resolved `handler` / `touches` paths, etc.); survey-derived blocks always include `kind: candidate`.

Example cross-source pub/sub candidate leaf:

```markdown
### identity.user-registration [acceptable, 894 LOC]

```yaml
kind: candidate
sources: [legacy-monolith, legacy-workers]
target_project: identity-svc
surfaces:
  - legacy-monolith:http-post-users
  - legacy-monolith:message-pub-user-created
  - legacy-workers:message-sub-user-created
evidence:
  citations:
    - legacy-monolith:src/auth/register.ts
    - legacy-workers:src/handlers/user_created.ts
  note: user.created
depends_on: [shared-validation]
```
```

Example cross-source HTTP-contract candidate leaf — caller in one source, route owner in another; `target_project` is inherited from the route owner per [Propagation Rules](#propagation-rules):

```markdown
### orders.checkout [acceptable, 612 LOC]

```yaml
kind: candidate
sources: [legacy-monolith, legacy-orders]
target_project: orders-svc
surfaces:
  - legacy-monolith:external-call-post-orders
  - legacy-orders:http-post-orders
evidence:
  citations:
    - legacy-monolith:src/checkout/api.ts:88
    - legacy-orders:src/routes.ts:14
  note: post-orders
depends_on: [identity.user-registration]
```
```

Re-running on unchanged inputs (including aliases) produces byte-identical `survey.md`.

### `identifier-aliases.yaml`

Operator-authored, tracked alongside the change. See [Identifier Normalization](#identifier-normalization) for schema, precedence, and validation.

## Mechanical Scanner

The CLI scanner invoked by `/change:analyze legacy-code`:

```text
specify change survey <source-path> --source-key <key> --out <dir>
```

The verb sits under `specify change` to make its plan-time role explicit and to keep the CLI namespace aligned with the rest of the change-lifecycle surface (`specify change draft`, `specify change finalize`, …).

It owns mechanical work only:

- Detect framework signatures.
- Enumerate surfaces.
- Resolve handlers and call sites where static analysis can do so.
- Record touched files.
- Validate and write `surfaces.json`.

The scanner does not call an LLM, infer candidates, or write `plan.yaml`.

**Flags.**

- `--source-key <key>` is **required**. The discovery brief always passes the key declared in `specify change draft --source <key>=<...>`; ad-hoc invocations must supply it explicitly. Synthesis is not duplicated across `analyze` and `survey` — failing closed surfaces mismatches immediately.
- `--out <dir>` is a directory. The verb always writes `surfaces.json` inside it (matching `analyze/<source-key>/`). If the directory already contains a `surfaces.json` whose `source_key` does not match `--source-key`, the verb exits non-zero rather than overwriting.
- `--format` is intentionally absent in v1. The output file is JSON by definition; the flag would re-introduce if and when stdout JSON envelopes are needed for shell pipelines.

**Exit discriminants.** Pinned set, kebab-case per the CLI repo's coding standards:

- `surface-scan-no-detectors-registered` — no detector applied to the source.
- `surface-scan-detector-id-collision` — two detectors emitted the same `surfaces[].id`.
- `surface-scan-source-path-missing` — `<source-path>` does not exist.
- `surface-scan-source-path-not-readable` — `<source-path>` cannot be read.
- `surface-scan-detector-failure` — a detector panicked or returned a malformed `Surface`.
- `surface-scan-alias-kind-invalid` — an alias bundle loaded at survey time failed the closed-kind enum check on `kind`.

No partial output is ever written; on any non-zero exit, `surfaces.json` is left untouched.

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
    framework_signatures: Vec<String>,
    surfaces: Vec<Surface>,
}
```

`Surface` matches the `surfaces.json` `surfaces[]` entry verbatim (including the structured `evidence` shape from [Artifacts](#artifacts)). Detectors return owned data; the verb deduplicates, sorts, and writes.

**Discovery rule.** `specify change survey` runs every registered detector against the source root. Each detector self-reports applicability: when its framework signatures are absent the detector returns an empty `DetectorOutput { framework_signatures: vec![], surfaces: vec![] }`. The verb:

1. Merges `framework_signatures` across all detectors (deduplicated, sorted).
2. Merges `surfaces` across all detectors and asserts no two detectors emitted the same `id`; on collision, exits `surface-scan-detector-id-collision`.
3. Validates the merged output against the `surfaces.json` schema and writes it atomically.

**Capability scoping.** v1 is a single global registry. Per-capability detector packs at `plugins/change/skills/survey/briefs/<cap>/detectors/` are explicitly deferred; the directory is reserved but not loaded in v1.

**Failure modes.** A detector that panics or returns a malformed `Surface` fails the run with `surface-scan-detector-failure`; the failing detector's name is included in the error payload so the operator can pin a workaround.

## Skill Responsibility Split


| Component                       | Responsibility                                                                                                                           |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `/change:analyze documentation` | Extract candidate hints from prose into `discovery.md`.                                                                                  |
| `/change:analyze legacy-code`   | Run the mechanical scanner; write `metadata.json` + `surfaces.json`; do not infer candidates.                                            |
| `specify change survey`         | Deterministically enumerate surfaces for one source.                                                                                     |
| `/change:survey`                | Compose all sources, run Pass 1 + Pass 2, size candidates, write `survey.md`, append candidate blocks under the discovery-owned heading. |
| `propose` brief                 | Ask the operator to accept/edit/reject candidates and write accepted plan entries through `specify plan add`.                            |


This split keeps expensive semantic judgement out of per-source analysis and lets cross-source clustering happen with the full system in view.

## Routing Hint Precedence

When assignment infers a target project for a survey-leaf slice, signals are consulted in strict order:

1. **Survey leaf `target_project`** — inherited from the nearest ancestor that carries one (typically a documentation hint, or the canonical owner on a cross-source leaf). Surfaced verbatim in the assignment table's `Rationale` column.
2. Description match (today's primary signal).
3. Baseline spec affinity (today's secondary signal).
4. Capability compatibility (today's tiebreaker).
5. Ambiguity → human.

Cross-source leaves carry the `target_project` of the canonical owner: publisher for pub/sub, route owner for HTTP, handler owner for WebSocket.

### Propagation Rules

`target_project` is propagated down the DAG by survey using a fixed set of rules so authors do not invent ad-hoc fallbacks:

1. **Doc-hint propagation.** If a documentation candidate with the same normalized name carries `target_project`, propagate it onto every cross-source or same-source leaf that mechanically maps to it.
2. **Canonical-owner propagation.** Cross-source leaves inherit `target_project` from the canonical owner. Pub/sub → publisher's source; HTTP → route owner's source (e.g. an `external-call-out` from `legacy-monolith` against an `http-route` owned by `legacy-orders` inherits `legacy-orders`' `target_project`); WebSocket → handler owner's source.
3. **Single-source, no-hint.** The leaf emits `target_project` as **absent** (not empty). Downstream assignment runs today's description-match → baseline-affinity → capability-compatibility chain. An absent field is legal and downstream-handled by design.
4. **Conflict.** If two ancestors carry conflicting `target_project` values (for example, a doc hint disagrees with a canonical owner), the leaf is marked `unresolved: true` with both candidates listed verbatim. Survey never silently picks a winner.

## Single-Source And Multi-Source Behavior

The algorithm is identical in both cases.

For a monolith, survey usually finds cross-module candidates: one candidate spanning several internal folders, workers, or packages.

For a repo fleet, survey can also find cross-source candidates: one candidate spanning multiple deployable systems connected by HTTP, messages, jobs, or shared external contracts.

The source count changes the breadth of the graph, not the planning model.

## Brownfield Behavior (v1)

When a target workspace already has `.specify/specs/` baselines, survey treats baseline projects as **opaque routing targets** consumed by existing assignment logic. It does **not** read baselines to flag delta-target opportunities; that is a propose-time concern with no concrete first user. See [Out Of Scope](#out-of-scope).

## Guardrails

- Survey is plan-time decomposition, not spec extraction. Full `spec.md` and `design.md` authoring still happens per slice through `/spec:define`, delegating to `/spec:extract` when legacy code is the source.
- Survey never writes `plan.yaml`. Only `specify change draft`, `specify plan add`, and `specify plan amend` write plan state.
- Legacy module boundaries are evidence, not authority. They may help find surfaces and code footprints, but they do not define slices.
- Unknown surface kinds, malformed sidecars, and aliases failing the closed-kind check fail closed.
- Outputs are byte-stable on unchanged inputs: fixed field order, sorted lists, no timestamps, no absolute paths, no host-specific state.
- Ambiguity is explicit. Survey emits `unresolved` candidates rather than inventing aliases or silently merging unrelated surfaces.

## Implementation Plan

1. Add the `surfaces.json` and `identifier-aliases.yaml` schemas + validators (closed-kind enforcement on alias `kind`; `evidence.citations` sorted, `evidence.note` matches the kebab-case grammar per [Artifacts](#artifacts)).
2. Add `specify change survey` with a stub detector registry, deterministic output, validation before write, the required `--source-key` flag, the `--out <dir>` directory contract, and the full exit-discriminant set documented in [Mechanical Scanner](#mechanical-scanner).
3. Land the framework identifier canonicaliser inside `/change:survey` with the rules in [Identifier Normalization](#identifier-normalization). Fixtures cover the canonical-form table and operator alias overrides on top of framework defaults.
4. Land the detector trait, `DetectorRegistry`, and `DetectorInput` / `DetectorOutput` shapes per [Detector Contract](#detector-contract), then the first mechanical detectors for the initial supported stack (Express, NestJS, BullMQ). Landing a real detector forces the contract to be exercised end-to-end.
5. Rewrite `/change:analyze legacy-code` to write `metadata.json` and `surfaces.json` only; rewrite `/change:analyze documentation` to emit candidate blocks in the unified fenced-YAML shape from [Artifacts](#artifacts).
6. **Combined release.** Land the discovery-brief edit that writes the `## Candidate inventory` heading wrapper *together with* the `/change:survey` skill in step 7 — the two must ship in a single PR ("discovery + survey heading handshake") to avoid a half-state where survey expects a heading the brief doesn't write.
7. Add `/change:survey` with Pass 1 (per-source size check; emit `acceptable` sources as single terminal candidates, hand `too_large` sources to Pass 2) and Pass 2 (intra-source clustering signals — `touches` overlap, framework module boundary, URI/topic prefix, worker-pool affinity — plus canonicalised cross-source pairing with canonical-owner rules, `consumes-external` annotation for unpaired subscribers, cycle detection on `depends_on`, and `unresolved: true` markers on `too_large` clusters that cannot be split). Wired against the thin evidence renderer from [Artifacts](#artifacts) (sorted citations + optional `note`). Wire it between workspace sync and propose.
8. Update `assignment.md` for the precedence in [Routing Hint Precedence](#routing-hint-precedence), including the propagation rules.
9. Acceptance fixtures (ship in step 7's PR or immediately after):
  - Single-source L monolith with one cross-module candidate.
  - Multi-source change with **≥ 3 source-keys** producing at least one cross-source candidate and one `unresolved` leaf resolved by adding to `identifier-aliases.yaml` and re-running survey.
  - Greenfield documentation-only pass-through (survey skipped entirely).
  - Single-source-already-S no-op (source is its own terminal candidate without further partitioning).
  - `too_large` cluster produced by intra-source clustering (heavy shared `touches`) that cannot be split and is emitted `unresolved: true` (Pass 2 fail-safe).
  - Subscriber with no in-scope publisher producing a `consumes-external` annotation (Pass 2 single-source fallback).
  - Two-source `depends_on` cycle that is resolved by aliasing two surface identifiers together (`cycle_with` markers, then clean topo order after re-run).
  - Alias bundle with an invalid `kind` value failing closed via `surface-scan-alias-kind-invalid`.
  - Fresh `/change:draft` end-to-end exercising the discovery-brief + survey handshake and asserting `## Candidate inventory` is emitted exactly once.
10. Tutorials: monolith decomposition and legacy-fleet decomposition (with one alias-resolved `unresolved`). Ship a stub `docs/explanation/legacy-migration-at-scale.md` alongside the tutorials, or defer the full document to the follow-on RFC that owns cross-change scale (RFC-21 / RFC-22 are the natural home).

## Migration

This is a plan-time behavioral change for legacy-code inputs.

**For operators.** `/change:analyze legacy-code` no longer infers candidate summaries directly into `discovery.md`. Instead it writes `metadata.json` + `surfaces.json` sidecars, and `/change:survey` owns candidate clustering and writes the candidate inventory for propose. In-flight plans do not need conversion — re-running `/change:draft` for a legacy-code change regenerates plan-time scratch artifacts in the new shape. Multi-source changes get cross-source clustering automatically; ambiguous identifiers surface as `unresolved` with the candidate set listed, and the operator extends `<plan-dir>/identifier-aliases.yaml` and re-runs.

**For capability authors.** Move the `legacy-code` clustering content out of `plugins/change/skills/draft/briefs/<cap>/analyze.md` into `plugins/change/skills/survey/briefs/<cap>/cluster.md`. `analyze.md` retains only the `documentation` branch and updates its emitted candidate block to the unified shape (`kind: candidate` plus the field set described in [Artifacts](#artifacts)). Surface detectors are registered as in-binary Rust detectors per [Detector Contract](#detector-contract); the `plugins/change/skills/survey/briefs/<cap>/detectors/` directory is reserved but not loaded in v1. Capability-owned alias overrides are deferred (see [Out Of Scope](#out-of-scope)); v1 supports a single operator-authored `<plan-dir>/identifier-aliases.yaml` layered on framework defaults.

**For skill authors consuming planning artifacts.** New artifacts: `surfaces.json` per source under `<plan-dir>/analyze/<source-key>/`, and `survey.md` under `<plan-dir>/`. Both schemas pinned, byte-stable. The `## Candidate inventory` heading in `discovery.md` is written exactly once by the discovery brief; both survey (legacy-code) and `/change:analyze documentation` append candidate blocks under it using the single fenced-YAML grammar defined in [Artifacts](#artifacts). Propose runs a single parser keyed on field names; missing fields default per the SKILL table.

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
| Per-capability `cut.md` brief separate from `cluster.md`                                                 | A capability author writes a Pass 1 refinement that materially exceeds half a page inside `cluster.md`.                                              |
| Per-capability `sizing.toml` overrides (tighten LOC rubric, add aggregate/endpoint counts)               | A capability demonstrates LOC-only sizing produces persistently wrong slices in operator review.                                                     |
| Per-capability `identifier-aliases.yaml` bundles merged into the framework / operator precedence         | A capability author repeatedly hand-copies the same alias rules into multiple plan-dir alias files.                                                  |
| Sub-source `group` DAG nodes and per-source structural depth cap                                         | A source produces enough surfaces that Pass 2 alone yields candidates the operator cannot review without intermediate framework-module structure.    |
| Finer-grained sizing buckets (XS/S/M/L/XL or similar)                                                    | Propose grows behavior that branches on more than the `acceptable` / `too_large` distinction (e.g. parallelism hints, review-effort budgeting).      |
| LLM-fallback detector contract and `--fallback-llm` flag                                                 | A real legacy stack outside the mechanical-detector envelope reaches the planning pipeline.                                                          |
| Brownfield reconciliation against `.specify/specs/` baselines (read baselines for delta-target flagging) | Brownfield-only changes reach the pipeline frequently enough that propose's missing delta-target awareness becomes a recurring complaint.            |
| Surface `confidence` field (graded high/medium/low)                                                      | The LLM-fallback contract lands; the field then differentiates mechanical from probabilistic detection.                                              |
| Closed `evidence.kind` discriminator (e.g. `framework-route`, `pubsub-pairing`, `http-pairing`, …)       | A consumer (propose, plan diffing, CI gate, telemetry) needs to branch on evidence category without re-deriving it from `surfaces[]` membership.     |
| Persisted `cross_module` / `cross_source` boolean flags on candidate leaves                              | A consumer needs to filter / branch on multi-module or multi-source candidates and the derivation from `sources` + namespaced `surfaces[]` proves expensive or error-prone in practice. |
| Machine-readable JSON sibling for `survey.md`                                                            | A downstream consumer (CI gate, registry sync, telemetry, plan diffing tool) needs structured survey data; v1 stays markdown-only.                   |


## References

- [RFC-13: Extensibility](archive/rfc-13-extensibility.md)
- [RFC-21: Source Catalogue and Tier-1 Cache](rfc-21-catalogue.md)
- [RFC-22: Migration Ledger](rfc-22-ledger.md)
- [RFC-23: Change Lifecycle](archive/rfc-23-change-lifecycle.md)
- `[/change:draft` SKILL.md](../plugins/change/skills/draft/SKILL.md)
- `[/change:analyze` SKILL.md](../plugins/change/skills/analyze/SKILL.md)

