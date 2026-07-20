# Adapter Descriptors and Registry Trust

> Status: Draft
>
> Owns: install-time adapter descriptors, the closed discovery vocabularies (including workload kinds), the registry descriptor projection, publisher and namespace trust policy, deterministic candidate filtering and explanation, and the shared recommendation-report currency.
>
> Depends on: [Self-Assembling Wasm Deployment](rfc-70-deployment.md).
>
> Consumed by: [Migration Intake and Source Selection](rfc-72-migration.md) (source selection) and [Migration Programs](rfc-74-program.md) (target selection).

## Abstract

Define the descriptor and trust substrate that lets Specify match adapters to inputs without hard-coded adapter names in engine core.

This RFC owns the layers common to both adapter axes:

1. a typed `AdapterDescriptor` authored beside each adapter and projected into the registry;
2. closed discovery vocabularies, including the workload-kind taxonomy;
3. deterministic candidate filtering against a repository profile or desired-state brief;
4. structured explanations, and the narrow conditions under which model judgment may adjudicate;
5. the immutable recommendation-report currency and its invalidation rules;
6. registry, publisher, and namespace trust policy.

Source selection — profiling inputs, composition, auto-bind conditions, and the `specify source recommend|approve` surface — is owned by [Migration Intake and Source Selection](rfc-72-migration.md). Target selection policy and Program Gate M1 are owned by [Migration Programs](rfc-74-program.md). A source adapter answers "what can faithfully inspect this input?" A target adapter answers "what should this project become?" Those are different questions with different owners; this RFC gives them one shared substrate.

## Motivation

The current adapter metadata is sufficient for execution but not discovery:

- source metadata carries only the Specify host-CLI floor;
- target metadata adds build inputs and a platform capability;
- path-based adapter detection is explicitly deferred;
- operators bind source adapters and target adapters by name.

That supports a small known adapter set. A migration intake containing unfamiliar repositories, multiple languages, design documents, screenshots, captures, or third-party adapters needs more.

Naive auto-selection would introduce two unacceptable risks:

- a model could choose and execute an arbitrary package from the network;
- Specify could treat the source implementation as the desired target architecture, for example choosing a web target merely because the legacy source is a web frontend.

Discovery therefore needs a typed descriptor, a deterministic inventory, and an explicit policy boundary — defined once and consumed by both the source-selection and target-selection surfaces.

## Goals

1. Describe adapter applicability in typed, searchable descriptors shared by first-party and third-party adapters.
2. Keep engine core free of adapter-specific `if name == ...` branches.
3. Make every recommendation explainable through matched evidence.
4. Keep registry search and package trust outside model control.
5. Give source selection and target selection one shared filtering, explanation, and report substrate.
6. Bind descriptors to exact component digests so an approval survives only while its inputs are unchanged.

## Non-goals

- Owning the recommendation and approval CLI surfaces. [Migration Intake and Source Selection](rfc-72-migration.md) owns the source surface; [Migration Programs](rfc-74-program.md) owns the target surface.
- Owning the repository profiler or the profile schema. [Migration Intake and Source Selection](rfc-72-migration.md) owns profiling; this RFC defines only the predicates evaluated against a profile.
- Letting an adapter mutate a repository during detection.
- Running every registry component to discover whether it might apply.
- Replacing source adapter `survey` or `extract`.
- Replacing target adapter `guidance`, `build`, or `merge`.
- Defining migration scheduling. [Migration Programs](rfc-74-program.md) owns scheduling.
- Making source language a sufficient target-selection rule.
- Allowing untrusted registries because a model recommends them.

## Decision

### Descriptor is authored with the adapter and projected into the registry

Each adapter defines one `AdapterDescriptor` beside its operation implementation. The adapter release pipeline projects the same value into the package registry index and binds it to the published component digest.

This preserves one authored home while making descriptors searchable before component installation.

**The descriptor is a sibling `describe` export — deferred until third-party adapters exist.** When the WIT surface grows, it grows as one deterministic `describe` operation per axis world: `metadata` keeps its small execution-critical shape (the host-CLI floor, a target's build inputs and platform capability), and `describe` carries discovery-only applicability data. Until then, first-party descriptors are ordinary Rust values in each adapter crate, and the release pipeline projects them into the static first-party index directly. The trigger for the export and the live registry projection is the first adapter whose descriptor Specify cannot compile in: a third-party component. Candidate generation reads the index or registry projection in either phase.

The Stage 1 authored form, illustratively — a plain value beside the operations implementor in the adapter's guest crate (`sources/typescript/` in `augentic/specify-adapters`), over closed SDK enums:

```rust
/// sources/typescript/src/descriptor.rs — projected into the static
/// first-party index by the release pipeline.
pub const DESCRIPTOR: AdapterDescriptor = AdapterDescriptor {
    axis: Axis::Source,
    stability: Stability::Stable,
    accepts: Accepts {
        media: &[Media::Repository],
        languages: &["typescript", "javascript"],
        frameworks: &["node", "deno", "react"],
        sentinels: Sentinel::AnyExists(&[
            "package.json", "deno.json", "deno.jsonc", "tsconfig.json",
        ]),
    },
    capabilities: Capabilities {
        evidence_kinds: &[
            EvidenceKind::Contract, EvidenceKind::Excerpt,
            EvidenceKind::Type, EvidenceKind::Call, EvidenceKind::Region,
        ],
    },
};
```

Package identity, component digest, WIT package, floor, and publisher are **not** authored fields: the release pipeline joins them from the crate version, the built component, and the publishing namespace when it projects the value, so a descriptor cannot claim an identity its component does not have.

The registry projection for a source adapter:

```yaml
schema-version: 1
package: specify:typescript@1.4.0
axis: source
component-digest: sha256:...
wit-package: specify:adapter/source@0.1.0
specify-floor: 0.27.0
publisher: augentic
stability: stable
accepts:
  media: [repository]
  languages: [typescript, javascript]
  frameworks: [node, deno, react]
  sentinels:
    any: [package.json, deno.json, deno.jsonc, tsconfig.json]
capabilities:
  evidence-kinds: [contract, excerpt, type, call, region]
```

A target projection uses desired-state vocabulary:

```yaml
schema-version: 1
package: specify:vectis@1.2.0
axis: target
component-digest: sha256:...
publisher: augentic
stability: stable
produces:
  workload-kinds: [mobile-app, cross-platform-ui]
  architectures: [crux]
  platforms: [core, ios, android]
requires:
  project-facts: [product-intent]
  optional-inputs: [tokens, assets, components, screenshots]
```

The descriptor is searchable applicability and distribution metadata. Operations still derive from the closed WIT axis contract.

### Closed discovery vocabularies

The initial schema uses closed values for facts that drive filtering:

- axis;
- media kind;
- workload kind;
- platform;
- stability;
- evidence kind;
- detector operator.

Languages, frameworks, architectures, and sentinel paths are normalized slugs with registry validation. New semantic axes require a schema revision rather than arbitrary metadata bags.

This RFC owns the **workload-kind taxonomy** consumed by descriptors, repository profiles ([RFC-72](rfc-72-migration.md)), and target policy ([RFC-74](rfc-74-program.md)). The initial closed set:

- `web-service` — network-facing service or API backend;
- `web-frontend` — browser-delivered UI;
- `mobile-app` — installable iOS or Android application;
- `desktop-app` — installable desktop application;
- `cli` — command-line tool;
- `library` — embedded library or SDK;
- `contract-set` — interface definitions (OpenAPI, AsyncAPI, JSON Schema) treated as first-class work.

Extending the set is a descriptor schema revision with registry validation.

Sentinel detection is intentionally limited to safe, declarative predicates:

- file or directory exists;
- one of a set exists;
- all of a set exist;
- a recognized manifest contains a dependency or package key;
- repository language share exceeds a threshold.

In the first-party phase these predicates are implemented as plain Rust detectors inside the profiler ([Migration Intake and Source Selection](rfc-72-migration.md)), each with a stable detector id. The declarative predicate schema is not evaluated from descriptor data until third-party adapters need to declare detection without shipping Rust — building a predicate interpreter for eight compiled-in adapters would be a rules engine without a second tenant. The list above is the ceiling either way: descriptors cannot execute scripts or arbitrary Wasm during candidate generation.

### Profile and brief inputs

Filtering evaluates descriptor predicates against two typed inputs this RFC does not own:

- a **repository profile** — deterministic, anchored observations about one source; the schema and profiler are owned by [Migration Intake and Source Selection](rfc-72-migration.md);
- a **desired-state brief** — migration intent, constraints, and organization policy facts; owned by [Migration Programs](rfc-74-program.md).

Both inputs are content-addressed. Every filtering result records the digests of the inputs it was computed from, so a changed profile or brief invalidates downstream recommendations deterministically.

### Candidate filtering

Candidate generation is deterministic:

1. query only configured registries and namespaces;
2. require the expected axis and WIT contract;
3. enforce the running Specify version floor;
4. enforce publisher, namespace, stability, and license policy;
5. evaluate descriptor predicates against the profile or desired-state brief;
6. reject candidates with unmet hard requirements;
7. return a stable ordered candidate set.

The policy consumed at steps 1 and 4 is a checked-in workspace document, illustratively:

```yaml
version: 1
registries:
  - name: first-party-static      # the Stage 1 index; a wasm-pkg registry later
    namespaces: [specify]
publishers:
  augentic: { stability: [stable] }
  acme-tools: { stability: [stable], license: [apache-2.0, mit] }
# Anything not named is not searched: an unlisted registry, namespace,
# or publisher is excluded at step 1/4, before predicates are evaluated.
```

No model call occurs before this filtering.

### Explanation and adjudication

Each candidate receives:

- matched facts;
- unmet soft preferences;
- blockers;
- trust result;
- registry identity and component digest;
- a human-readable rationale generated from the structured result.

There is deliberately **no numeric ranking framework in the first version** — no normalized scores, configured thresholds, or ambiguity margins. Deterministic filtering either yields one candidate, which is eligible for policy-gated auto-binding, or several, which always go to adjudication or operator review. The tuning apparatus is deferred until a real migration produces an ambiguity that coarse filtering plus adjudication cannot settle; adding a score to the report shape then is additive.

One candidate's explanation, as `specify adapter explain` projects it for the `legacy-billing` repository profile from [Migration Intake and Source Selection](rfc-72-migration.md):

```yaml
candidate: specify:typescript@1.4.0
axis: source
component-digest: sha256:8c2d…
trust:
  registry: first-party-static
  publisher: augentic
  stability: stable
  result: pass
matched:
  - fact: accepts.media
    value: repository
    evidence: profile.media
  - fact: accepts.languages
    value: typescript
    evidence: profile.languages[typescript].share=0.72
  - fact: accepts.sentinels.any
    value: [package.json, tsconfig.json]
    evidence: profile.sentinels
unmet-preferences: []
blockers: []
rationale: >
  Sole surviving source candidate for media `repository`: the profile's
  dominant language and sentinel files match the descriptor's accepted
  set, and no other configured candidate accepts this media kind.
```

Every `matched` entry pairs a descriptor fact with the profile or brief anchor it matched — the "evidence-backed" property is this structure, not the prose `rationale`, which is generated from it.

A model may adjudicate only when:

- several candidates survive deterministic filtering;
- semantic intent in design documents is needed;
- one repository carries several independently useful source modalities;
- target intent is underspecified or contradictory.

The model cannot add a candidate that deterministic filtering rejected and cannot weaken trust policy.

Model adjudication is a judgment leg like survey or synthesis: its answer shape is pinned by a `recommendation` schema in the committed answers-goldens family (`crates/project/answers/`), generated from the same Rust wire types the deterministic tail parses. The model selects among the deterministically ranked candidates and cites the evidence behind the selection; the deterministic tail rejects any answer naming a candidate outside the filtered set.

### Recommendation reports

Discovery produces an immutable, CLI-owned recommendation report containing:

- input digests (profile or brief);
- registry query and trust policy versions;
- candidates with matched facts, blockers, and trust results;
- ambiguities and missing desired-state facts;
- proposed exact selectors and component digests;
- approval status.

An illustrative report for the same source, before approval:

```yaml
report: rec-2026-07-19-legacy-billing-01
axis: source
inputs:
  profile: sha256:4f15…        # RepositoryProfile digest (RFC-72)
  registry-index: first-party-static@0.30.0
  trust-policy: sha256:77aa…
candidates:
  - package: specify:typescript@1.4.0
    component-digest: sha256:8c2d…
    trust: pass
    matched: [accepts.media, accepts.languages, accepts.sentinels.any]
    blockers: []
  - package: specify:documentation@1.1.0
    component-digest: sha256:aa17…
    trust: pass
    matched: [accepts.media]
    blockers: []
    note: scoped to profile artifact `docs/` — a distinct binding, not a rival
ambiguities: []
proposed:
  - binding: code
    selector: specify:typescript@1.4.0
    component-digest: sha256:8c2d…
  - binding: docs
    selector: specify:documentation@1.1.0
    component-digest: sha256:aa17…
    subpath: docs
approval:
  status: pending
```

The consuming surface (`specify source approve` here, Program Gate M1 for targets) flips `approval.status` and records the actor; any change to `inputs.profile`, the policy digest, a descriptor, or a candidate's component digest invalidates the report in place.

The report is immutable once approved. A changed profile, policy, descriptor, or component digest invalidates the report and any approval derived from it; re-running discovery creates a replacement report.

The report **writers** are the consuming surfaces: source recommendations and approvals belong to [Migration Intake and Source Selection](rfc-72-migration.md); target recommendations and Program Gate M1 belong to [Migration Programs](rfc-74-program.md). In every case approval lowers into the existing single writers — source bindings into `plan.yaml.sources` or the source catalogue, target bindings through `specify init` — and package identities are hydrated through `Resolver::ensure_*`. Hydration and dispatch are one path: once `ensure_*` writes verified bytes into the store (or project component cache), Omnia's registry-miss guest resolver ([Self-Assembling Wasm Deployment](rfc-70-deployment.md)) loads them on first call — no regenerated static guest list, and no Specify vocabulary inside Omnia.

### CLI surface

Read-only inspection lives with this substrate:

```bash
specify adapter search --axis source --profile <profile>
specify adapter search --axis target --intent <brief>
specify adapter explain <package>
```

Mutation does not. `specify source recommend|approve` ([RFC-72](rfc-72-migration.md)) and `specify migration inspect|approve` ([RFC-74](rfc-74-program.md)) own recommendation and approval; every approval records the actor and exact selected identities in the journal.

## Registry and package requirements

The current hydration protocol can fetch bytes for a known exact identity. Discovery additionally requires:

- an index query by axis and descriptor fields;
- immutable descriptor documents bound to component digests;
- publisher identity and signature or equivalent registry provenance;
- yanked and superseded version signals;
- compatibility with the configured wasm-pkg namespace routing;
- offline index snapshots for reproducible and air-gapped use.

Registry search is read-only. Installation remains an exact-identity operation.

These registry capabilities are required by the third-party adapter ecosystem (roadmap RM-21), **not** by the first migration program. The migration walking skeleton runs entirely on the static first-party index from Stage 1 below; registry discovery is gated on third-party adapters existing, not on [RFC-74](rfc-74-program.md).

If the package ecosystem adopts OCI or Warg-backed discovery, this RFC's descriptor and policy semantics remain unchanged; only the registry client changes.

## Implementation stages

### Stage 1 — First-party descriptors and static index

1. Add the descriptor types to the adapter SDK as plain Rust values; no WIT change.
2. Define descriptors for every first-party adapter.
3. Validate descriptor identity against crate and component identity.
4. Generate a static first-party index for development, tests, and the migration walking skeleton.
5. Add deterministic candidate filtering (Rust detectors) and explanation.

This stage is the only prerequisite this RFC imposes on the first migration program.

### Stage 2 — Recommendation reports and adjudication

1. Add the structured explanation currency.
2. Add recommendation-report persistence and digest-based invalidation.
3. Add the `recommendation` answers schema and the model adjudication leg.

### Stage 3 — Registry discovery (gated on the third-party ecosystem)

1. Add the `describe` export to the WIT axis worlds and the SDK export macros.
2. Publish digest-bound descriptor projections from the `describe` export.
3. Evaluate declarative sentinel predicates from descriptor data.
4. Add configured-registry search.
5. Enforce publisher and namespace trust policy.
6. Support offline index snapshots.
7. Hydrate only after approval — dispatch then follows the RFC-70 miss-hook.

## Acceptance criteria

1. Workflow core contains no adapter-name matching branches.
2. When the `describe` export ships, it is discovery-only; execution continues to read `metadata`.
3. Every recommendation cites matched profile or desired-state evidence.
4. A model cannot introduce a package that deterministic filtering excluded.
5. The static first-party index and a registry index produce identical candidate sets for identical inputs.
6. Installation verifies the descriptor's component digest.
7. A changed profile, policy, descriptor, or component digest invalidates the derived report and approval.
8. An offline index can reproduce candidate generation without network access.
9. First-party and third-party adapters use the same descriptor schema.

## Testing

- Descriptor validation, candidate filtering, and explanation are deterministic and model-free: crate-level integration tests over the static first-party index, following the integration-first posture.
- The adjudication leg runs against scripted answers from the mock catalogue in native integration tests; the `recommendation` schema joins the answers-goldens parity gate.
- Adjudication prompt quality is an eval scenario (`cargo make eval scenario`), not a CI assertion.
- The Stage 3 registry client is tested over an offline index snapshot fixture; no test performs live registry queries.

## Open questions

1. Which package provenance mechanism is required before third-party auto-install can be enabled?
2. How should descriptor schema evolution relate to the WIT package version?
