# RFC-24: Omnia Plan Composition

> **Status: Deferred.** Reconcile with the current lead/evidence reconciliation model before implementing: the `surfaces` digest is derived from slice `model.yaml` / `spec.md` (with `discovery.md` leads as the plan-time signal) or authored directly on the plan entry — there is no `surfaces.json` sidecar. Verify the referenced Omnia skill names (`omnia-crate-writer`, `omnia-guest-writer`) and brief paths against the live `adapters/targets/omnia/` tree. The load-bearing design — two additive optional plan-entry fields (`crate`, `surfaces`), five Omnia-gated validate findings, and the read-only `specify plan compose` join — holds against the current plan schema. Backs the roadmap's ["Omnia plan composition"](../roadmap.md#ideas-parked) parked idea.
>
> Depends: [RFC-21](rfc-21-catalogue.md) (source catalogue), [RFC-22](rfc-22-ledger.md) (ledger), the current plan schema (`schemas/plan/plan.schema.json`), and the Omnia target adapter briefs (`adapters/targets/omnia/briefs/`).

## Abstract

Teach `plan.yaml` how to express the composition shape Omnia migrations actually produce — *services* composed of *crates* composed of *handlers* — without inventing a parallel artifact and without breaking any existing plan.

Today the mapping is implicit: an Omnia "service" is whatever `project` an Omnia target adapter uses; a "crate" is whatever the slice ends up named by convention in the Omnia build brief; "handlers" only show up downstream in `spec.md` after source evidence is folded into the slice. Operators cannot review the service-shape of a planned migration without opening `discovery.md`, three slice directories, and the project registry side by side.

This RFC adds:

1. **An optional `crate` field on `plan.yaml.slices[]` entries** - explicit override of the slice -> crate name mapping, defaulting to `slice_name` with kebab-to-snake conversion.
2. **An optional `surfaces` field on `plan.yaml.slices[]` entries** - audit-only handler digest (kind counts plus, optionally, normalised identifiers) lifted from source `survey` leads (plan time) / slice `model.yaml` (refine time) by `/spec:plan`.
3. **Target adapter discriminated `specify plan validate` findings for Omnia-typed projects** - handler-identifier uniqueness within a service, publisher uniqueness per topic, and a closed-enum check that every declared surface kind is one Omnia can host.
4. **`specify plan compose --project <name>`** - a derived read verb that joins `plan.yaml`, per-slice source evidence, and `registry.yaml` to render the service -> crate -> handler tree on demand.

All four pieces are strictly additive. Existing plans validate without change, the slice loop is unaware, execution is unchanged, and nothing here writes to artifacts that another RFC owns.

## Motivation

The survey step produces a source-derived inventory rich enough to drive planning: each lead can carry `target_project`, `surfaces[]`, `touches[]`, `depends_on`, and `evidence`. RFC-21 makes those leads cheap to recompute across changes. RFC-22 records the cross-change audit trail. None of them give the *operator-reviewable plan* a way to express the composition shape of an Omnia target.

For an Omnia-only migration this matters in three concrete ways:

- **Service composition is invisible.** `plan.yaml` says which slices land in `identity-svc`, but not what `identity-svc` ends up looking like — how many crates it grows, what HTTP routes / topics / WebSocket channels it now owns. Operators reverse-engineer this by reading `discovery.md`, the project's existing `.specify/specs/`, and any prior changes' archives.
- **Crate identity is convention-only.** `omnia-crate-writer` hard-wires `$CRATE_PATH = crates/$CRATE_NAME` and `$CRATE_NAME = $ARGUMENTS[0]` — i.e. the slice name. Any escape hatch (a slice whose canonical crate name differs from the slice name, or a slice whose name is operator-friendly but illegal as a Rust crate ident) requires hand-editing after the writer runs.
- **Cross-slice handler conflicts surface late.** Two slices in the same Omnia project can both declare `POST /users`, both publish to `user.created`, or both own the `/ws/notifications` channel. The conflict is discoverable only at guest-wiring time, deep in `/spec:build` — long after the operator could have re-scoped slices cheaply during plan review.

None of these are blockers; all three are friction. This RFC removes the friction with the smallest additive surface that keeps every existing invariant.

## Design

### Principles

1. **Service identity is the project, full stop.** An Omnia service is a `project` in `registry.yaml` whose configured target adapter resolves to `omnia` (any version). This RFC adds no new field for "service".
2. **Crate identity is the slice, by default.** The slice -> crate 1:1 mapping is already the contract the Omnia build brief enforces; this RFC formalises it as the schema default and adds a single optional override.
3. **Handlers stay where they are.** Slice `model.yaml` and `spec.md` remain the sources of truth for handler/surface detail (with `discovery.md` leads as the plan-time signal). `plan.yaml` carries an *audit-only digest*, mirroring RFC-22's `mapping` posture: it is summary metadata for operator review, not a re-encoding of the evidence.
4. **The validator is target adapter discriminated.** None of the new findings fire for projects whose resolved target adapter is not Omnia. Vectis, contracts, and greenfield-without-target projects validate exactly as today.
5. **The CLI is the single writer.** New fields are written only by `specify plan create`, `specify plan add`, and `specify plan amend`. No skill hand-edits them.
6. **Schemas are strict.** `additionalProperties: false`, kebab-case identifiers, closed surface-kind enum.
7. **No execution-time branching.** The slice loop, `/spec:execute`, `/spec:refine`, `/spec:build`, and `/spec:merge` ignore the new fields. They are review and audit signal only.

### `crate` field on `plan.yaml.slices[]`

Additive optional field on `schemas/plan/plan.schema.json` for each `plan.yaml.slices[]` entry.

```yaml
slices:
  - name: identity-user-registration
    project: identity-svc
    crate: user_registration         # optional; default = name with kebab-to-snake conversion
    sources: [legacy-monolith]
    status: pending
    description: Extract registration handler and email-verification publisher.
```

Schema:

```json
"crate": {
  "type": "string",
  "pattern": "^[a-z][a-z0-9_]*$",
  "description": "Rust crate name produced by this slice. Snake_case; matches `^[a-z][a-z0-9_]*$`. Defaults to the slice name with kebab-to-snake conversion when absent. Read by the Omnia build brief to set `$CRATE_PATH = crates/<crate>` and by `specify plan compose` to render service composition."
}
```

Rules:

- **Optional.** Every existing plan validates as today.
- **Default derivation.** When absent, `crate` is computed as `slice.name.replace('-', '_')`. The default is *not* materialised into `plan.yaml` — only operator-set values are stored, mirroring how `depends-on: []` is omitted rather than written explicitly.
- **Validation.** Pattern is the standard Rust crate-name pattern (snake_case, must start with a letter). Crate names must be unique *within a single project* across the plan; collisions are an error finding (`plan-omnia-crate-name-collision`). Cross-project collisions are legal — `auth` in `identity-svc` and `auth` in `gateway-svc` are different workspaces.
- **Writer consumption.** `omnia-crate-writer` reads `crate` (falling back to the default derivation) instead of assuming `$ARGUMENTS[0]` equals the crate name. The writer's contract becomes: "use the slice's `crate` field; reject if the field is absent and the slice name is not a legal crate ident".
- **Target adapter gated.** Validation fires only when the slice's resolved target adapter is Omnia. For non-Omnia projects the field is allowed (forward compatibility) but produces no findings.

### `surfaces` field on `plan.yaml.slices[]`

Additive optional field on each `plan.yaml.slices[]` entry. Carries the audit-only handler digest.

```yaml
slices:
  - name: identity-user-registration
    project: identity-svc
    crate: user_registration
    sources: [legacy-monolith]
    surfaces:
      - kind: http-route
        identifier: POST /users
      - kind: message-pub
        identifier: user.created
      - kind: message-sub
        identifier: email.verified
    status: pending
```

Schema (closed-enum):

```json
"surfaces": {
  "type": "array",
  "description": "Handler digest for Omnia-typed slices. Audit-only; derived from source `survey` leads / slice `model.yaml` via `/spec:plan`, validated by `specify plan validate`. The slice loop and execute do not branch on this field. Identifiers are the canonicalised form from source evidence, not the verbatim source identifier.",
  "items": {
    "type": "object",
    "additionalProperties": false,
    "required": ["kind", "identifier"],
    "properties": {
      "kind": {
        "type": "string",
        "enum": [
          "http-route",
          "message-pub",
          "message-sub",
          "ws-handler",
          "scheduled-job"
        ]
      },
      "identifier": {
        "type": "string",
        "minLength": 1
      }
    }
  },
  "uniqueItems": true
}
```

Rules:

- **Optional.** Every existing plan validates as today. Hand-driven plans (no survey run) may omit the field entirely; `specify plan compose` then falls back to the count-only summary derived from the slice's `model.yaml` / `spec.md`.
- **Closed kind enum.** `cli-command`, `ui-route`, and `external-call-out` are deliberately excluded - Omnia services do not host CLI entry points or UI routes, and `external-call-out` is an outbound dependency rather than a hosted surface. A lead containing those kinds either splits its surfaces (keeping only Omnia-hosted ones in `plan.yaml.surfaces`) or fails the target adapter gated validate finding `plan-omnia-surface-kind-unsupported`.
- **Identifier shape.** Stored as the canonicalised form per the surface identifier-normalization rules so cross-slice comparison is deterministic. The original identifier is preserved verbatim in the slice's `model.yaml`; `plan.yaml` carries only the matching key.
- **Sorted.** `surfaces[]` is sorted by `(kind, identifier)` for byte-stable diffs, same posture as `migration-log.yaml`.
- **Plan populates it.** When `/spec:plan` accepts a lead whose target project resolves to Omnia, it pre-fills `surfaces[]` from the lead's `surfaces:` list, intersected with the Omnia kind enum. Operators may amend via `specify plan amend <slice> --add-surface <kind>:<identifier>` / `--remove-surface <kind>:<identifier>`. Passing `--clear-surfaces` removes the field entirely.

### Adapter-gated `specify plan validate` findings

Five new findings, all gated on the slice's resolved target adapter being Omnia:

| Code | Severity | Trigger |
|---|---|---|
| `plan-omnia-crate-name-collision` | error | Two slices in the same project resolve to the same `crate` name (operator-set or default-derived). |
| `plan-omnia-crate-name-illegal` | error | `crate` field absent and slice name is not a legal Rust crate ident (numbers leading, reserved keyword, etc.). |
| `plan-omnia-surface-kind-unsupported` | error | `surfaces[].kind` is outside the closed Omnia enum (e.g. a `ui-route` smuggled in by hand-edit). |
| `plan-omnia-handler-conflict` | error | Two slices in the same project declare the same `(kind, identifier)` for `http-route`, `ws-handler`, or `scheduled-job`. |
| `plan-omnia-publisher-conflict` | warning | Two slices in the same project declare `kind: message-pub` for the same `identifier`. Warning rather than error because consolidation patterns sometimes legitimately fan out a single topic across crates during migration; the warning forces operator acknowledgement. |

`message-sub` deliberately has no uniqueness finding — multiple subscribers per topic across crates within one service is a legal and common Omnia pattern.

All findings live within `EXIT_VALIDATION_FAILED=2` per [the CLI exit-code contract](../../engine/AGENTS.md#exit-codes). Findings carry the standard structured shape (`code`, `severity`, `slice`, `project`, `evidence`).

### `specify plan compose` — derived composition view

A new read-only verb that joins `plan.yaml`, the per-slice `model.yaml` for any in-progress slice, `registry.yaml`, and (when present) `migration-log.yaml`.

```bash
specify plan compose                           # tree for every Omnia-typed project in the plan
specify plan compose --project <name>          # tree for one project
specify plan compose --format json             # machine-readable envelope
```

Default human output:

```text
identity-svc (omnia@1)
├── user_registration  [pending]
│   ├── http: POST /users
│   ├── pub: user.created
│   └── sub: email.verified
├── shared_validation  [done]
│   └── (no hosted surfaces)
└── email_verification [in-progress]
    ├── http: POST /verify
    └── sub: user.created
```

JSON envelope:

```json
{
  "version": 1,
  "projects": [
    {
      "name": "identity-svc",
      "target-adapter": "omnia@1",
      "crates": [
        {
          "name": "user_registration",
          "slice": "identity-user-registration",
          "status": "pending",
          "surfaces": [
            { "kind": "http-route", "identifier": "POST /users" },
            { "kind": "message-pub", "identifier": "user.created" },
            { "kind": "message-sub", "identifier": "email.verified" }
          ]
        }
      ]
    }
  ]
}
```

Rules:

- **Read-only.** Like other read-only inspection surfaces, it never writes to `plan.yaml`.
- **Fallback when `surfaces` is absent.** If a slice has no `surfaces[]` field but a current `model.yaml` exists for the slice, the verb falls back to deriving the digest from that file. If neither is available the crate row prints `(surfaces unknown)`; this is not an error.
- **Cross-RFC composition.** When [RFC-22's ledger](rfc-22-ledger.md#-specifymigration-logyaml--the-cumulative-ledger) is present, finalised crates from prior changes are listed below the in-flight ones under a `# Previously migrated` heading. This makes `compose` the single answer to "what does this Omnia service look like, today, and after this change lands?".

### Adapter resolution

The validator and `plan compose` both need to know whether a slice's `project` is Omnia-typed. The resolution rule is:

1. If `registry.yaml` exists and has a `projects[]` entry matching the slice's `project`, the project's configured target adapter decides. Match is by prefix: `omnia`, `omnia@1`, `omnia@2`, etc. all gate the Omnia validator on.
2. If the slice has no `project` (project-less target-bound slice), the slice's `target` field decides.
3. If neither resolves to Omnia, none of the new findings or `compose` rendering fires for that slice. The fields are still schema-valid; they just produce no Omnia-specific behaviour.

This keeps the RFC scoped to its stated case ("targeting Omnia solely") without preventing mixed-target plans from validating. A plan containing one Omnia service and one Vectis app is fine; only the Omnia slices get the new findings.

## Implementation Plan

1. **Schemas.** Land the `crate` and `surfaces` fields on `schemas/plan/plan.schema.json` for `plan.yaml.slices[]` entries (`additionalProperties: false`, kebab-case, closed Omnia kind enum). Update `schemas/plan/README.md`. Add JSON Schema fixtures: happy-path, collision, illegal-kind, missing-crate-default-derived.
2. **Domain types.** Extend `specify-workflow`'s plan entry type with `crate_name: Option<String>` and `surfaces: Vec<PlanSurface>`. `serde(deny_unknown_fields)`. Add a `resolved_crate_name(&self) -> Result<String, Error>` helper that applies the kebab-to-snake default and validates the Rust crate-ident shape.
3. **Target adapter resolution.** Add `Plan::resolve_target_adapter(&Entry, &Registry) -> Option<TargetAdapterRef>` in `specify-workflow`. Mirror the resolution rule in §Adapter resolution; cover registry-present, registry-absent, and `target`-on-slice paths.
4. **Validator findings.** Extend `Plan::validate` with the five target adapter gated findings. New discriminants in `specify-error`: `plan-omnia-crate-name-collision`, `plan-omnia-crate-name-illegal`, `plan-omnia-surface-kind-unsupported`, `plan-omnia-handler-conflict`, `plan-omnia-publisher-conflict`.
5. **`specify plan amend` extensions.** Accept `--crate <name>`, `--clear-crate`, `--add-surface <kind>:<identifier>`, `--remove-surface <kind>:<identifier>`, `--clear-surfaces`. Atomic write through the existing `AtomicYaml` trait. Same single-writer rule as every other plan field.
6. **Omnia build integration.** Update the Omnia build brief to read `crate` from the slice's plan entry when running inside `/spec:execute` instead of assuming `$ARGUMENTS[0]`. The brief stays driver-agnostic: if invoked outside a plan loop the positional `$CRATE_NAME` still works.
7. **`specify plan compose`.** New read verb under `src/commands/plan/compose.rs`. Walks `plan.yaml` slices, resolves the target adapter per slice, fills `surfaces[]` from the plan field or falls back to the slice's `model.yaml` when present. JSON envelope sibling of `plan-validate-output`.
8. **Plan brief integration.** Update `/spec:plan` to pre-fill `crate` (defaulting to kebab-to-snake of the accepted slice name) and `surfaces` (intersected with the Omnia enum) when the lead's `target_project` resolves to an Omnia project. Adapter authors who own non-Omnia briefs do nothing.
9. **Survey integration.** No source adapter schema changes - the lead's surface inventory is already the source. The lead's `surfaces:` block is what `/spec:plan` copies in.
10. **Acceptance fixtures.** Extend the cross-repo acceptance suite with: an Omnia-only plan with two slices in the same project that collide on `POST /users` (error); two slices that both publish `user.created` (warning); a slice whose default-derived crate name is illegal (`123-invalid` -> finding); a mixed-target plan (one Omnia slice + one Vectis slice) asserting no findings fire for the Vectis slice; a `specify plan compose --format json` snapshot.
11. **Tutorials.** Add `docs/tutorials/omnia-migration-composition.md` walking through `/spec:plan` -> `plan compose` for an Omnia-only multi-source migration. Update `docs/tutorials/legacy-migration-at-scale.md` to reference the new fields.

## Migration

This RFC is **strictly additive**. Pre-existing plans, surveys, registries, archives, and changes continue to work without change.

**For operators.** Existing `plan.yaml` files validate without modification. After upgrade, `/spec:plan` against an Omnia target pre-fills the new fields; re-running plan amendments on an in-flight change leaves operator-set fields alone. `specify plan compose` works against any plan, falling back to the slice's `model.yaml` when `surfaces[]` is absent.

**For target adapter authors.** Only the Omnia target adapter's plan/build guidance changes - to populate and consume `crate` and `surfaces`. Vectis, contracts, and any third-party target adapter are unaffected. The new validator findings are guarded by the Omnia target adapter check, so non-Omnia projects see no behavioural change.

**For skill authors consuming planning artifacts.** The plan-validate JSON envelope gains five new finding codes within the existing `EXIT_VALIDATION_FAILED=2` exit. `specify plan compose --format json` is a new readable surface with a stable JSON envelope; treat it like a read-only inspection surface. The Omnia build brief consumes the new `crate` field via direct plan inspection rather than relying on argument convention.

There is **no breaking change** to: existing `plan.yaml` files (both new fields are optional), existing `registry.yaml` files (untouched), existing exit codes (new discriminants within `EXIT_VALIDATION_FAILED=2`), or any non-Omnia target adapter brief.

## Alternatives Considered

**Add an Omnia-shaped sub-document under each slice (`omnia: { crate: ..., surfaces: [...] }`).** Rejected. A target adapter scoped namespace inside `plan.yaml.slices[]` would invite Vectis, contracts, and every future target adapter to claim their own sub-document, fragmenting the schema and forcing skill authors to learn a per-adapter vocabulary inside the same file. Top-level optional fields with target adapter gated validation cost less and compose better; the validator already needs to know the project's target adapter for routing-hint precedence anyway.

**Carry `surfaces` as count-only (`{ http: 1, message-pub: 1, message-sub: 1 }`).** Considered. Counts give the at-a-glance size signal but drop the cross-slice uniqueness check, which is the main operational reason to lift handlers into the plan at all. Identifiers cost slightly more diff churn and pay for themselves the first time `plan validate` catches two slices binding `POST /users`.

**Skip `surfaces` entirely; rely on `specify plan compose` to derive everything from the slice's `model.yaml`.** Rejected. Slice `model.yaml` is slice-time evidence: it does not exist until a slice is refined and is not co-located with `plan.yaml`, so deriving the digest on demand makes plan review depend on slice-time state. Lifting the digest into `plan.yaml` makes it durable, diff-reviewable, and amendable through the same single-writer flow as every other plan field.

**Allow multiple crates per slice (`crates: [a, b]`).** Rejected. The slice loop's existing invariant is one slice -> one phase artifact set; the Omnia build brief assumes one crate per slice. Multi-crate slices would force `/spec:build` to interleave generation across crates, complicating both the writer and the build review loop with no clear win. Operators who need two crates author two slices.

**Generalise the validator findings beyond Omnia (e.g. cross-target handler-conflict).** Rejected for v1. Vectis's "service" is a shared-core Crux app with platform shells, not a hosted-handler workspace; contracts targets have no hosted handlers; neither would benefit from the Omnia uniqueness checks. The target adapter gated validator keeps this RFC tightly scoped; analogous companion RFCs for Vectis or contracts can mirror the structure if and when those target adapters grow comparable composition concerns.

**Encode crate names as kebab-case (matching the slice name shape) and convert at writer time.** Rejected. Rust crate identifiers are snake_case; recording the canonical Rust form in `plan.yaml` keeps the field's meaning concrete and removes a silent transformation step from the writer. The kebab-to-snake default derivation preserves operator convenience for the common case.

**Make `crate` required when the project target adapter is Omnia.** Rejected for v1. The default derivation handles the vast majority of slices; requiring it would force every Omnia plan written before this RFC to amend before it re-validates, which violates the additivity principle.

## Non-Goals

- Driving execution from `plan.yaml.surfaces`. The slice loop continues to read handlers from `spec.md` via `omnia-crate-writer` and `omnia-guest-writer`; the plan-level field is review and audit signal only.
- Multi-crate-per-slice generation. The one-slice-one-crate contract is intentional and preserved.
- Per-handler dependencies (a `depends-on` at handler granularity). Slice-level `depends-on` remains the only ordering surface.
- Vectis or contracts composition. Those target adapters have different deployable shapes; if either grows comparable plan-time concerns, a companion RFC mirroring this one is the right move.
- Replacing slice `model.yaml` / `spec.md` as the source of truth for handler detail. `plan.yaml.surfaces` is a digest, not a replacement.
- Cross-change handler-conflict detection. Within a plan, the validator catches collisions; across changes, the conflict is RFC-22's ledger territory and falls out of `specify plan compose` rendering.
- A `specify plan compose --diff` mode comparing the planned composition to the project's current `.specify/specs/` baseline. Useful but unscoped; defer until operator demand is concrete.
- LLM-assisted crate naming or surface grouping. Both fields are mechanical: defaults derive deterministically, `/spec:plan` copies from leads verbatim, operators amend explicitly.

## Open Questions

1. Should `crate` and `surfaces` move under a single optional `omnia` sub-document anyway, on the theory that the per-target-adapter namespace becomes self-explanatory once two or three adapters populate it? Current preference: no - keep them top-level optional, let the validator's target adapter gate carry the discrimination, and revisit if a third target adapter materially benefits from a similar lift.
2. Should the `plan-omnia-publisher-conflict` finding be an error rather than a warning, on the theory that a single topic legitimately owned by two crates inside one service is always an architectural smell? Current preference: warning, because plan-time often reflects in-flight refactors where a topic temporarily has two publishers; operators can promote the warning to an error in their CI gate if they want stricter posture.
3. Should `specify plan compose` also render `cross_source` and `cross_module` flags from survey, or is that overkill for an Omnia-shaped tree? Current preference: skip them — `compose` is a service-composition view, not a survey replay; operators wanting that detail open `discovery.md`.
4. Should the surface-kind enum include `health-check` or `metrics-endpoint` as separately-tracked Omnia kinds? Current preference: no — both are framework-level concerns belonging to the guest wrapper, not slice-owned handlers; a slice should not "own" `/healthz` in `plan.yaml`.
5. Should the `crate` field permit `crates/`-relative subpaths (e.g. `internal/auth`) for projects organised into nested crate trees? Current preference: no — flat `crates/<name>/` is the contract `omnia-crate-writer` and `omnia-guest-writer` agree on; nested workspaces are a separate, larger change.
6. Should `/spec:plan` populate `surfaces[]` only when the lead's confidence is high (in some future world where the survey lead grows a `confidence` field), or always? Current preference: always when present - the field is audit-only, and a `surfaces[]` written from a low-confidence lead is no worse than an empty one.
7. Should the Omnia build brief reject when the slice's resolved target adapter is *not* Omnia (e.g. the operator pointed the writer at a Vectis slice by mistake)? Current preference: yes - fail-closed with a clear discriminant, since the writer's output is wasm32-target-specific and cannot meaningfully apply to a non-Omnia project.

## References

- [`engine/docs/standards/workflow.md`](../../engine/docs/standards/workflow.md) and [From sources to slices](../../plugins/spec/references/reconciliation.md) — the survey/lead and extract/evidence flow the surface digest sources from.
- [RFC-21: Source Catalogue and Source-Clone Cache](rfc-21-catalogue.md) — `sources.yaml` and the source-clone cache.
- [RFC-22: Migration Ledger and Slice Mapping](rfc-22-ledger.md) — the audit-only-field precedent (`mapping`) and the ledger this RFC's `plan compose` overlays for previously-migrated crates.
- [`adapters/targets/omnia/briefs/build.md`](../../adapters/targets/omnia/briefs/build.md) — the build brief whose `$CRATE_NAME` contract this RFC formalises into a schema field, and which owns the Omnia service workspace into which handlers land.
- [`schemas/plan/plan.schema.json`](../../schemas/plan/plan.schema.json) — the schema this RFC additively extends with `crate` and `surfaces`.
- [`schemas/target.schema.json`](../../schemas/target.schema.json) — target adapter manifest format the validator consults for the Omnia gate.
- [`engine/crates/workflow/src/registry/catalog.rs`](../../engine/crates/workflow/src/registry/catalog.rs) — registry project metadata source the target adapter resolution reads.
- [`engine/AGENTS.md#exit-codes`](../../engine/AGENTS.md#exit-codes) — the exit-code contract the new findings respect.
