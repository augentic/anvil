# RFC-27: Synthesis Sharpening

> Status: Implemented. Additive to [RFC-25](rfc-25-workflow.md); supersedes nothing. Lifts the four sharpening recommendations in [rfc-25-synthesis.md](rfc-25-synthesis.md) into normative decisions. Compatible with [RFC-19](rfc-19-observability.md) (new journal events), [RFC-21](rfc-21-catalogue.md) (`sources.yaml`), and [RFC-24](rfc-24-omnia.md) (`shape` is unchanged).

## Abstract

RFC-25 put the slice boundary on disk before any LLM writes a `spec.md`, which is the right operator-trust seam for code-generation accuracy on multi-source migrations. RFC-27 keeps that spine and sharpens the four places the synthesis review identified as load-bearing weak points without redesigning either fusion layer:

1. **A first-class runtime source adapter.** Promote `plugins/rt/`'s wiretap-and-replay pattern into a `adapters/sources/captures/` source adapter. Runtime captures become `Evidence` with a new `kind: example` claim and default authority `behaviour`. Targets MAY consume the same captures during `build` and record results in `.metadata.yaml`; v1 keeps the replay hook optional and operator-visible, with no automatic `merge` refusal.
2. **Authority widens beyond the closed 3-class enum.** Authority becomes a property of `(Evidence document, claim kind)` rather than a property of the Evidence document alone, with per-slice operator overrides in `plan.yaml`. The current `intent > documentation > behaviour` ordering stays as the default per kind; per-slice overrides land at Gate 1, not after synthesis.
3. **A thin reconciliation index.** Each `/spec:refine` writes `.specify/slices/<slice>/fusion.yaml` listing every `REQ-*` id in `spec.md` and the contributing `(source-key, claim-id)` pairs plus the authority outcome. One inspectable artifact per slice; no graph database.
4. **Smaller-but-load-bearing fixes.** Move `divergence: likely` writes entirely into the CLI; add an `aliases: []` field on candidate blocks; add a `specify plan create --auto-review` flag that stamps `lifecycle: reviewed` atomically with create on any plan shape; define cache fingerprints explicitly and journal them on every `extract`.

All four are strictly additive against RFC-25. Existing plans, slices, evidence files, adapters, and skills validate without change. The new behaviours sit on optional fields and a new (additive) source adapter that ships disabled until an operator binds it.

```text
source adapters --enumerate--> discovery.md (Candidate{aliases?}) --propose--> plan.yaml (slices[].divergence?, --auto-review at create)
                                                                                  |
                                                                                  v
        +-- captures --extract--> evidence/captures.yaml (kind: example, authority: behaviour)
        +-- documentation -extract--> evidence/<doc-key>.yaml
        +-- code-typescript -------> evidence/<legacy-key>.yaml
                                                                                  |
                                              core synthesis (authority-resolved per kind, per-slice overrides honoured)
                                                                                  |
                                                                                  v
                                              proposal.md / spec.md / design.md / tasks.md
                                              fusion.yaml (REQ -> contributing claims + values + outcome)
                                                                                  |
                                                                                  v
                                              adapters/targets/<name>/build (MAY run generated code against captures from the source binding; results recorded in .metadata.yaml; merge is operator-visible, not auto-refusing)
```

## How to read this RFC

| Audience                | Start here                                                          |
| ----------------------- | ------------------------------------------------------------------- |
| Operator / skill author | §Operator surface → §Authority widening → §Reconciliation index     |
| Source adapter author   | §Runtime source adapter → §Candidate aliases → §Cache fingerprints  |
| Target adapter author   | §Runtime source adapter (`build` consumption) → §Migration          |
| CLI implementer         | §Normative decisions → §Implementation contract → §Implementation plan |
| RFC-25 reviewer         | §Motivation → §Alternatives considered                              |

## Motivation

[rfc-25-synthesis.md](rfc-25-synthesis.md) was a three-way best-of-N review of RFC-25's two-step `enumerate → propose → extract → synthesize` spine. The verdict was unanimous: keep the two-step. The review then identified exactly four leverage points where the spine is structurally correct but ergonomically or auditably under-powered. Each is concrete enough to design against:

- **Behavioural evidence is parallel, not on the spine.** The `plugins/rt/` wiretapper and replay-writer already produce the strongest form of evidence Specify can consume (captured production I/O), but they live in a sibling plugin and feed `tests/data/replays/` rather than `evidence/*.yaml`. Synthesis cannot cite a capture as a `Sources:` entry, and `build` cannot refuse to merge when generated code fails fixture replay. This is the single biggest code-gen accuracy lever still on the table — see §Runtime source adapter.
- **Authority is too coarse.** Closed 3-class enum declared per *Evidence document* means "docs always win over code," which is wrong at least half the time on legacy migrations where production behaviour is the truth. Per-claim-kind precedence and per-slice override are explicitly deferred in [`authority.md`](../plugins/spec/references/synthesis/authority.md) line 105 ("Per-claim or per-slice authority overrides are deferred — there is no `authority-override` field on slice entries or claims in v1"); they will be needed sooner than the v1 posture admits. See §Authority widening.
- **Synthesis decisions are opaque after the fact.** Once `/spec:refine` writes `spec.md`, the operator's only audit path is "open every `evidence/*.yaml` and reconstruct the join in your head." A flat index that lists every `REQ-*` id and the contributing `(source-key, claim-id)` pairs is the smallest change that makes synthesis surprises debuggable. See §Reconciliation index.
- **Writer-ownership, candidate joins, N=1 ergonomics, and cache trust are all off-spine in small but corrosive ways.** [`plugins/spec/skills/plan/SKILL.md`](../plugins/spec/skills/plan/SKILL.md) line 43 documents the skill as the sole writer of `divergence: likely` because the CLI rejects it on `plan amend` — the exception erodes the "CLI is the single writer of `plan.yaml`" invariant. `candidate.id` does double duty as stable handle and join key. N=1 pays three CLI calls and a context switch for "fix a typo." Cache hits today mean "byte-stable goldens," not "byte-stable production runs across model versions." See §Smaller fixes.

None of these requires redesigning either fusion layer. All four are strictly additive at the schema and CLI level.

## Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Runtime source adapter** | Ship `adapters/sources/captures/` as a first-party source adapter that consumes the RT plugin's runtime captures. Build-time fixture replay is an optional target hook in v1. | New `kind: example` claim in `evidence.schema.json`; RT skills rehome behind the adapter surface; targets MAY add a fixture-replay step to `build` and record results into `.metadata.yaml`. `merge` does not auto-refuse on replay failure in v1. |
| **D2 Per-kind authority** | Authority becomes a property of `(Evidence document, claim kind)`. Default ordering stays `intent > documentation > behaviour` per kind. | `evidence.schema.json` gains an optional `authority-overrides: { <claim-kind>: <authority> }` map; synthesis fusion table consults the per-kind value when resolving disagreement. |
| **D3 Per-slice authority override** | `plan.yaml.slices[]` carries an optional `authority-override: { <claim-kind>: <source-key> }` map honoured by synthesis. | Schema additive; `specify plan amend --authority-override <slice> <claim-kind>=<source-key>` lands the value; `specify slice validate` rejects orphan source keys. |
| **D4 Reconciliation index** | `/spec:refine` writes `.specify/slices/<slice>/fusion.yaml`. | New `schemas/slice/fusion.schema.json`; one entry per `REQ-*` id in `spec.md`; `specify slice fusion show <slice>` reads it. |
| **D5 CLI owns `divergence: likely`** | `specify plan create --divergence-likely <slice>` and `specify plan amend --divergence likely` accept the value end-to-end. | The `plan` skill stops re-reading and re-writing `plan.yaml` to append the field. |
| **D6 Candidate aliases** | `## Candidate inventory` blocks carry an optional `aliases: [<kebab>, ...]` field; `slices[].sources[].candidate` resolves against `id` OR any `alias`. | `candidate.schema.json` additive; `specify plan add --sources <key>=<id-or-alias>` accepts either; aliases inspectable via `specify discovery show`. |
| **D7 Auto-review at create** | `specify plan create --auto-review` stamps `lifecycle: reviewed` atomically with create. Valid on any plan shape; the flag *is* the operator's Gate-1 consent at create time. | New flag; emits the same `plan.transition.reviewed` journal event the post-create stamp would, in a single atomic journal append with `plan.create`. |
| **D8 Cache fingerprints** | Every `extract` cache lookup and write keys on the fingerprint `(source path canonicalised, adapter name@version, brief sha256, declared-tool versions, candidate id)`. | New `.specify/.cache/adapters/sources/<adapter>/index.jsonl` append-only log; new `slice.extract.cache-hit` / `.cache-miss` journal events carrying the fingerprint. |

All eight decisions land in the same minor release (Specify 2.1). No 2.0 manifests, plans, or evidence files require changes to validate.

## Operator surface

What changes for an operator running the default rhythm:

```bash
/spec:init <target>
/spec:plan <name> source design=./design-notes source legacy=./vendor/monolith source runtime=./captures/replays
specify plan transition <name> reviewed     # unchanged
/spec:execute                                # unchanged
/spec:finalize <name>                        # unchanged
```

Two new convenience commands cover the most-used ergonomic paths:

```bash
specify plan create <name> --auto-review --intent "fix typo in user.rs"   # D7
specify plan amend <name> <slice> --authority-override requirement=legacy  # D3
specify slice fusion show <slice>                                          # D4
```

And one new inspection verb:

```bash
specify discovery show --aliases   # D6
```

The default rhythm is unchanged at every other touchpoint. `captures` is a normal source adapter the operator binds explicitly; nothing turns on without the binding.

## Runtime source adapter (D1)

### Adapter shape

```yaml
# adapters/sources/captures/adapter.yaml
name: captures
version: 1
axis: source
operations: [enumerate, extract]
briefs:
  enumerate: briefs/enumerate.md
  extract:   briefs/extract.md
tools:
  - name: fixture-index
    declared: wasi-tools/fixture-index
```

The adapter is loaded by `crates/domain/src/adapter/` through the same `Adapter::resolve(Axis::Source, "captures", project_dir)` code path as `code-typescript` and `documentation` (RFC-25 §Default source adapters). No core branch.

### Binding

Operators bind a runtime capture directory the same way they bind a legacy code path:

```yaml
# plan.yaml fragment
sources:
  runtime:
    adapter: captures
    path: ./captures/replays
```

`$SOURCE_DIR` is the bound directory, read-only, under the same sandbox RFC-25 §Sandboxing defines for every source adapter (no `$PROJECT_DIR`, no host env, no network). The directory layout the adapter expects matches [`adapters/sources/captures/references/capture-format.md`](../../adapters/sources/captures/references/capture-format.md) — `tests/data/replays/<handler>/<scenario>.json` plus optional `samples/`. Operators with a non-conforming layout adapt the directory or write a thin wrapper adapter; v1 does not invent a new capture format.

### `enumerate` output

One candidate per observed entry point (HTTP route, message handler, scheduled job, WebSocket handler). The block grammar matches RFC-25 §Discovery handshake verbatim:

```markdown
### user-registration

- id: user-registration
- sources: [runtime]
- summary: POST /users observed in 47 captures; one publishes user.created.
```

When `captures` is bound alongside `code-typescript` and a documentation source, the per-source candidate ids will not necessarily match across sources. The new `aliases: []` field (D6) covers that case explicitly. Pre-existing candidate-id heuristics (kebab-case noun phrase, RFC-25 §Discovery handshake) continue to apply.

### `extract` output

A new `kind: example` claim joins the `claim-kind` enum:

```yaml
# .specify/slices/identity-user-registration/evidence/runtime.yaml
source: runtime
adapter: captures
authority: behaviour
candidate: user-registration
claims:
  - kind: example
    claim-id: users.register.happy-path
    path: tests/data/replays/users-register/happy.json
    fixture-digest: sha256:7a2b...
    input:
      method: POST
      route: /users
      body: { email: alice@example.com, password-hash: "$argon2..." }
    output:
      status: 201
      side-effects:
        - kind: message-pub
          topic: user.created
          payload-shape: { user-id: uuid, email: string }
    statement: "Registering with a fresh email returns 201 and publishes user.created with the new user-id."
```

Schema additions (additive against [`schemas/evidence.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/evidence.schema.json)):

- `claim.kind` enum gains `example` (placed alphabetically; existing enum value order is irrelevant to validation but golden-file order is preserved).
- `claim.fixture-digest`, `claim.input`, `claim.output` properties are intentionally open per the existing per-kind body posture; `fixture-digest` MUST start with `sha256:`.
- `claim.fixture-digest` is the fingerprint the cache (D8) keys against.
- `claim-id` is required on `example` claims (same posture as `requirement` / `criterion`) so the fusion table can resolve agreement across sources.

The adapter MUST NOT emit capture bodies larger than 64 KiB inline; over-budget captures carry only `fixture-digest` and `path`. The 64 KiB limit lives in the adapter brief, not the schema, so larger limits are reachable through a fork without a schema change.

### Default authority and Sources lines

`authority: behaviour` is the document-level default for every `captures` Evidence file. The per-kind override map (D2) is rarely needed for this adapter — runtime captures are behaviour by definition. The synthesis fusion table (§Synthesis updates) gives `example` claims the same default precedence as `excerpt` and `call` claims from `code-typescript`: `behaviour`-class, beaten by `documentation` and `intent` for `requirement`-kind disagreements unless a per-slice override (D3) flips the ordering for the slice.

`spec.md` requirements cite the `runtime` source key in `Sources:` the same way any other source key is cited:

```markdown
### Requirement: Password reset request

ID: REQ-001
Sources: [product-notes, legacy-monolith, runtime]
Status: agreed

The system lets a registered user request a password reset link by email.
```

The order rule from RFC-25 §Authority hierarchy ("highest authority first") is preserved per *kind*. For a `requirement` block where the contributing claims include `documentation`-class and `behaviour`-class contributors, `product-notes` precedes `legacy-monolith` and `runtime` in the list. For a `criterion` block where the operator has flipped precedence via per-slice override, the list reflects the override.

### `build`-time fixture replay

`adapters/targets/<name>/build` MAY consume the same captures the `captures` adapter extracted from. When a target's `build` brief implements the hook, it MUST record results in the slice's `.metadata.yaml` under a `fixture-replay` block:

```yaml
# .specify/slices/<slice>/.metadata.yaml fragment
fixture-replay:
  passed: 47
  failed: 0
  skipped: 0
  ran-at: 2026-05-22T13:18:42Z
  runner: omnia-target@1.4 (cargo nextest)
```

Rules:

- The block is *additive* to `.metadata.yaml`; targets that have not implemented the hook omit the field entirely. Omission is not an error.
- `merge` reads the field if present and surfaces a one-line summary in its closing message (`fixture-replay: 47 passed, 0 failed`) so the operator notices replay failures before they push. `merge` does **not** auto-refuse on `failed > 0` in v1 — the operator decides whether to land a slice whose generated code does not pass runtime captures, the same way RFC-25 leaves `[conflict]` and `[divergence]` tags as review signals rather than gates.
- Operators who want stricter posture can wire `merge` refusal into their target adapter (a custom Omnia fork can refuse on `failed > 0` from its own brief) or into their CI gate (`specify slice outcome show <slice> --format json` exposes the block). A future RFC may promote auto-refusal into core if real consumers ask; v1 stays advisory to match the rest of the synthesis-tag posture.
- The fixture-runner shape depends on the target (Omnia generated crates run replay tests through `cargo nextest`; contracts targets run through `specify tool run contract`; Vectis targets do not consume captures). Each target chooses whether and how to invoke the hook.

The fixture-runner is not new code. Today's [`plugins/rt/skills/replay-writer/`](../plugins/rt/skills/replay-writer/SKILL.md) already wires this up for Omnia targets; D1 reuses the same skill body, invoked from `adapters/targets/omnia/briefs/build.md` instead of as a sibling plugin step. The RT plugin's two skills become thin wrappers over the source adapter's `enumerate` / `extract` once D1 lands; the wiretapper retains its TypeScript-instrumentation role since instrumentation is not a Specify-spine concern.

The optional posture follows RFC-24 §`surfaces[]` precedent: target-specific structured outputs are recorded for operator review without core branching on their values. Promoting fixture-replay to a hard `merge` gate is a single-line change in a future RFC if v1 telemetry shows operators consistently want it.

## Authority widening (D2 + D3)

### Per-kind precedence on Evidence

`evidence.schema.json` gains an optional `authority-overrides` map keyed by claim kind. The map values are the same closed enum the document-level `authority:` field uses.

```yaml
source: legacy-monolith
adapter: code-typescript
authority: behaviour            # document-level default; applied when no per-kind override matches
authority-overrides:
  decision: documentation       # decisions extracted from comments outrank decision claims sourced elsewhere
  criterion: behaviour          # explicit; pins behaviour-class precedence even if a future default changes
candidate: user-registration
claims:
  - kind: excerpt
    claim-id: users.register.email-validation
    path: src/users/register.ts#L12-L87
```

Rules:

- The document-level `authority:` field stays required. `authority-overrides` is purely additive — an Evidence file without the field behaves exactly as today.
- The override map is a *closed-enum-to-closed-enum* lookup. Both keys (claim kinds) and values (authority classes) reuse the existing schema enums; new kinds or classes still require an RFC update.
- The override applies to *all* claims of the named kind in that Evidence document. Per-claim overrides remain explicitly deferred (see §Non-goals).
- Synthesis consults `authority-overrides` first, falls back to the document-level `authority:` second, falls back to the RFC-25 default ordering third. The fallback chain is byte-stable.

### Per-slice override on the plan

`plan.yaml.slices[]` gains an optional `authority-override` map keyed by claim kind and valued by source key:

```yaml
slices:
  - name: identity-user-registration
    target: omnia
    project: identity-svc
    sources:
      - key: identity-design-notes
        candidate: user-registration
      - key: legacy-monolith
        candidate: user-registration
      - key: runtime
        candidate: user-registration
    authority-override:
      requirement: runtime        # runtime captures dictate requirement-class disagreements on this slice
      criterion: legacy-monolith  # legacy code dictates criterion-class disagreements
    status: pending
```

Rules:

- The override map is scoped to the slice. Plan-wide and project-wide overrides are out of scope.
- Keys are the closed claim-kind enum. Values are source keys that MUST be present in the slice's own `sources[]` list — synthesis rejects an orphan key with structured error `slice-authority-override-orphan-source-key`.
- When a `requirement` block's contributing claims disagree at the same authority class after Evidence-level overrides resolve, the per-slice override picks the winning source key directly. Resolution order:
    1. Per-slice `authority-override.<kind>` matches a contributing source key → that source wins.
    2. Per-Evidence `authority-overrides.<kind>` resolves a per-kind authority that breaks the tie → that class wins.
    3. RFC-25 default ordering on document-level `authority:` → highest wins.
    4. Still tied → `Status: conflict` and `[conflict]` tag, unchanged from RFC-25.

The override surface is intentionally narrow. It does not change the closed enum, it does not add per-claim controls, and it does not give the operator a way to invent a fourth authority class. The migration use case the synthesis review called out — "legacy code is often production truth" — resolves to one line per slice in `plan.yaml`.

### CLI surface

```bash
specify plan amend <plan> <slice> --authority-override <claim-kind>=<source-key>
specify plan amend <plan> <slice> --clear-authority-override <claim-kind>
specify plan amend <plan> <slice> --clear-authority-overrides
```

`specify plan add` accepts the same flags repeated. `specify slice validate` rejects orphan source keys before `/spec:refine` runs.

### Acceptance rule for the spec.md provenance parser

`crates/domain/src/spec/provenance.rs` already cross-resolves `Sources:` entries against `plan.yaml.slices[].sources[]`. D2 + D3 add one additional check: when a `Status: divergence` block lists multiple `Sources:` entries, the parser MUST report which override (if any) selected the winning source. The check is informational — the parser does not refuse on missing overrides — but the resolution path is recorded in the per-slice `fusion.yaml` (D4) so the operator can audit it later.

## Reconciliation index (D4)

`/spec:refine` writes one new file per slice: `.specify/slices/<slice>/fusion.yaml`. The file is byte-stable across runs given the same inputs and is the operator's single audit surface for synthesis decisions.

```yaml
# .specify/slices/identity-user-registration/fusion.yaml
version: 1
slice: identity-user-registration
generated-at: 2026-05-22T13:15:00Z
generator: specify@2.1.0
requirements:
  - id: REQ-001
    status: agreed
    sources: [identity-design-notes, runtime]
    contributing-claims:
      - source: identity-design-notes
        claim-id: password-reset.request
        kind: requirement
      - source: runtime
        claim-id: users.register.happy-path
        kind: example
    resolution: single-value-agreement
  - id: REQ-007
    status: divergence
    sources: [identity-design-notes, legacy-monolith]
    contributing-claims:
      - source: identity-design-notes
        claim-id: password-reset.expiry
        kind: criterion
        value: "Reset links expire after 30 minutes."
        path: docs/account.md#L7
        winner: true
      - source: legacy-monolith
        claim-id: password-reset.expiry
        kind: criterion
        value: "expiresAt = createdAt + 24h"
        path: src/users/reset.ts#L42
        winner: false
    resolution: authority-resolved
    resolution-trace:
      step: per-slice-authority-override
      override: { criterion: identity-design-notes }
      winner: identity-design-notes
  - id: REQ-008
    status: conflict
    sources: [product-notes, identity-design-notes]
    contributing-claims:
      - source: product-notes
        claim-id: password-reset.expiry
        kind: criterion
        value: "Reset links expire after 30 minutes."
        path: docs/product/reset.md#L12
      - source: identity-design-notes
        claim-id: password-reset.expiry
        kind: criterion
        value: "Reset links expire after 60 minutes."
        path: docs/identity/reset.md#L4
    resolution: tied-conflict
```

Schema (lives at `schemas/slice/fusion.schema.json`, additive):

- `version` is a stored integer, currently `1`.
- `slice` MUST match the directory name.
- `generated-at` is a UTC ISO-8601 timestamp; the resolution is to the second, not the nanosecond (byte-stable diffs across reasonably-fast clocks).
- `generator` is the CLI version that wrote the file.
- `requirements[]` carries one entry per `REQ-*` id in `spec.md`. Order matches `spec.md` order.
- `contributing-claims[]` lists every `(source, claim-id)` pair the synthesis consulted, *not* only the winning one. Operators auditing a divergence can see what was dropped.
- Each `contributing-claims[]` entry carries the load-bearing claim payload inline: `value` (the single-line statement / criterion / decision body the source asserted), `path` (the `<path>#L<n>` anchor copied from the source Evidence claim), and `winner` (boolean; present and `true` on the entry that synthesis selected; present and `false` on entries dropped by authority resolution; absent on entries from `agreed` blocks where there is no winner / loser distinction). The inline payload means the operator can read the full disagreement in `fusion.yaml` alone for the common review case; opening `evidence/*.yaml` is only required when the claim's per-kind body carries fields beyond `value` (e.g. `example` claims with `input` / `output` blocks).
- `value` is a single-line string. Multi-line claim bodies are truncated to the first non-empty line in `fusion.yaml`; the truncation indicator `"…"` appears at the end, and the full body stays in the source `evidence/*.yaml` (linked by `path`). The 16 KiB cap on a single `value` field is enforced by the writer; over-cap values truncate at a whitespace boundary and append `"…"`.
- `resolution` is a closed enum: `single-source`, `single-value-agreement`, `authority-resolved`, `per-slice-override`, `unknown-no-evidence`, `tied-conflict`.
- `resolution-trace` is optional and present only when `resolution` is `authority-resolved` or `per-slice-override`; it names the step that broke the tie.

Rules:

- `/spec:refine` writes `fusion.yaml` atomically (same atomic-write posture as `plan.yaml`); a partial write never appears on disk.
- `/spec:refine` re-running on a slice overwrites `fusion.yaml`. The file is regenerated whole; no hand-edits survive (parallel to `spec.md` re-refine semantics from RFC-25).
- `specify slice validate` MUST verify that every `REQ-*` id in `spec.md` has an entry in `fusion.yaml` and vice versa, and that every `contributing-claims[]` entry's `(source, claim-id, path)` triple resolves to a real claim in the per-source `evidence/<source-key>.yaml`. Drift on either check produces structured error `slice-fusion-drift`.
- `specify slice fusion show <slice>` prints a human-readable rendering (the inline `value` payloads are what the human-readable form prints by default); `--format json` emits the file verbatim.
- The reconciliation index is *not* an authoritative input to any downstream verb. `spec.md` remains the authoritative artifact for `/spec:build`; `fusion.yaml` is audit-only, in the same spirit as RFC-22's `mapping` field and RFC-24's `surfaces[]`.

## Smaller fixes

### D5 — `divergence: likely` CLI ownership

Two changes:

1. **`specify plan create` and `specify plan amend` accept `--divergence likely`.** The CLI is the single writer of `plan.yaml.slices[].divergence` across every value of the closed enum, not just `accepted` / `rejected`.
2. **`plugins/spec/skills/plan/SKILL.md` step 3 is rewritten** to invoke the CLI rather than re-reading and patching `plan.yaml`. The `## Likely divergences` block in `change.md` is unaffected — the skill continues to author the operator-facing prose; only the YAML write moves.

The journal event `plan.propose.divergence` (already in `crates/domain/src/journal.rs`) now fires from CLI code paths only; the skill's `append`-from-body call moves into `specify plan amend --divergence likely`'s normal event emission. No event-payload schema changes.

This is the only D5 change. The on-disk shape of `plan.yaml` is unchanged.

### D6 — Candidate aliases

`schemas/discovery/candidate.schema.json` gains an optional `aliases: []` field:

```json
"aliases": {
  "type": "array",
  "description": "Optional kebab-case aliases for cross-source merge. `slices[].sources[].candidate` resolves a binding's candidate against the candidate's `id` or any `alias`. Authored by the source adapter at `enumerate` time or by `/spec:plan` at `propose` time when merging differently-named candidates across sources. Stable across re-enumeration.",
  "items": { "$ref": "#/$defs/kebabName" },
  "uniqueItems": true
}
```

`## Candidate inventory` block grammar gains one optional bullet:

```markdown
### user-registration

- id: user-registration
- aliases: [account-registration, user-signup]
- sources: [legacy-monolith, runtime]
- summary: Registration endpoint accepting email + password with RFC-5322 validation.
```

Rules:

- Aliases are kebab-case, the same shape as `id`. Empty `aliases: []` and a missing field are equivalent.
- `slices[].sources[].candidate` resolves first against `id`, then against `aliases[]`. Resolution is case-sensitive.
- Re-enumeration replaces a candidate by `id`; the adapter MAY emit a different `aliases[]` list on the new write, but operator-authored alias additions through `specify plan amend` survive (see below).
- An alias MUST NOT collide with any other candidate's `id` or `aliases[]` in the same `discovery.md`; the collision is a `discovery-alias-collision` error from `specify slice validate`.

CLI surface:

```bash
specify plan add <plan> <slice> --sources <key>=<id-or-alias>
specify plan amend <plan> <slice> --add-alias <candidate-id>=<alias>
specify plan amend <plan> <slice> --remove-alias <candidate-id>=<alias>
specify discovery show --aliases
```

`specify plan add --sources <key>=<alias>` is shorthand: the CLI rewrites the value to the resolved `id` before persisting, so `plan.yaml.slices[].sources[].candidate` always carries the canonical `id`. Aliases are stored only in `discovery.md`.

### D7 — Auto-review at create

`specify plan create --auto-review` stamps `lifecycle: reviewed` atomically with create. The flag *is* the operator's Gate-1 consent — typing it acknowledges that the operator has reviewed every `--sources`, every `--intent`, and every `--target` argument they passed on the same command line. RFC-25 §D6 ("Gate 1 only") is preserved: review still happens between plan and execute; it just happens in one CLI invocation rather than two.

Rules:

- Valid on any plan shape — N=1 pure-intent, N=1 path-bound, or N>1 with multiple sources. There is no pre-condition on slice count, source count, or `key: intent` presence.
- The CLI emits both `plan.create` and `plan.transition.reviewed` (existing payload, unchanged) in a single atomic append to `.specify/journal.jsonl`. Downstream consumers see the same event sequence as the two-call path.
- `--auto-review` and explicit post-create `specify plan transition <name> reviewed` are mutually exclusive in normal use, not enforced as such — running the transition on an already-`reviewed` plan is the existing CLI no-op.
- Plans with `slices[].divergence: likely` set by `propose` still validate under `--auto-review`. The operator explicitly opted into reviewing the plan with that field present; the field's review-signal posture from RFC-25 §Plan-time fusion is unchanged.
- `--auto-review` does **not** bypass plan validation. The CLI runs the same `plan validate` it runs on the post-create path; validation failures refuse the create, with or without the flag.

Worked examples:

```bash
# Degenerate N=1 — the synthesis review's original motivating case
specify plan create fix-typo --auto-review \
    --intent "fix typo in user.rs" \
    --target omnia

# N>1 hand-authored plan — operator typed every binding, no propose ceremony to review
specify plan create identity-revamp --auto-review \
    --source identity-design-notes=./design-notes/identity \
    --source legacy-monolith=./vendor/monolith \
    --slice identity-user-registration --target omnia --project identity-svc \
        --sources legacy-monolith=user-registration \
        --sources identity-design-notes=user-registration \
    --slice identity-password-reset --target omnia --project identity-svc \
        --sources legacy-monolith=account-pwd-reset \
        --sources identity-design-notes=password-reset
```

Both invocations exit at `lifecycle: reviewed` with `/spec:execute` immediately legal. The operator's review is the act of writing the create command; the flag asserts that act.

The flag is intended for two operator profiles:

- **Trivial N=1** — fix a typo, add a comment, rename a function. The synthesis review's original motivating case.
- **Hand-authored multi-slice** — the operator already knows the slice shape (e.g. from a prior dry-run plan they discarded, or from a runbook they're following) and is typing it directly. Forcing them to run `specify plan transition` after typing every binding has no review benefit.

Operators who want the agent-led `propose` pass to inform their review continue to use the two-call path: `specify plan create <name> [bindings]` → read `discovery.md` + `change.md` + `plan.yaml` → `specify plan transition <name> reviewed`. The two paths produce byte-identical final state.

### D8 — Cache fingerprints

Today's `.specify/.cache/adapters/sources/<adapter>/` is a content-addressed scratch directory; cache reuse across runs is implicit and relies on adapter authors' good faith. D8 makes the fingerprint explicit and auditable.

Fingerprint inputs (closed list, order-stable):

1. Canonical absolute path of `$SOURCE_DIR` (or the `value:` body's sha256 for value-style bindings).
2. Adapter name and version from `adapter.yaml`, joined as `<name>@<version>`.
3. sha256 of the brief markdown file that drove the operation (`briefs/enumerate.md` or `briefs/extract.md`).
4. The declared-tool versions used during the operation, sorted by tool name (matches the `tools[]` declaration in the manifest).
5. The candidate id (`extract` only; absent for `enumerate`).

The CLI joins these into a single sha256-keyed cache entry:

```text
.specify/.cache/adapters/sources/<adapter>/<fingerprint>/
    evidence.yaml          # or candidate-set.md, for enumerate
    fingerprint.json       # full input record for audit
```

A new append-only log at `.specify/.cache/adapters/sources/<adapter>/index.jsonl` records every cache write with the fingerprint, the slice it served, and the journal event id. `specify source resolve --explain <adapter>` reads the log and prints the fingerprint chain for the operator.

New journal events (added to `crates/domain/src/journal.rs`):

| Event | Payload | When |
| --- | --- | --- |
| `slice.extract.cache-hit` | `{ slice, source-key, adapter, fingerprint }` | Cache lookup matched; `extract` was not re-run. |
| `slice.extract.cache-miss` | `{ slice, source-key, adapter, fingerprint, reason }` | Cache lookup missed; `extract` ran. `reason` is one of `no-prior-entry`, `source-path-changed`, `adapter-version-changed`, `brief-sha-changed`, `tool-version-changed`. |

These events are the basis for CI's "trust cached evidence across model versions" decision. CI that pins the four fingerprint inputs at a known set can re-run any prior `/spec:execute` and expect byte-stable cache hits; CI that observes any of the five `reason` values knows exactly which input drifted.

D8 does not change adapter brief shape. Adapters keep returning content through CLI-mediated paths; the fingerprint is computed and recorded by the CLI without any adapter cooperation. Adapter authors who want to opt their cache out of fingerprinting (e.g. for non-deterministic intermediate results) declare `cache: opt-out` in their `adapter.yaml`; the CLI then treats every run as a cache miss and the index log carries `reason: adapter-opt-out`.

## Implementation contract

### Schema deltas

All schemas live in `augentic/specify-cli/schemas/` per RFC-25 §Repository layout.

| Schema | Change | Decision |
| --- | --- | --- |
| `evidence.schema.json` | Add `example` to `claimKind` enum. Add optional top-level `authority-overrides: { <claim-kind>: <authority-enum> }`. Add open per-claim fields `fixture-digest`, `input`, `output`. | D1, D2 |
| `discovery/candidate.schema.json` | Add optional `aliases[]`. | D6 |
| `plan/plan.schema.json` | Add optional `planSlice.authority-override: { <claim-kind>: <source-key> }`. Document that `divergence: likely` is CLI-written. | D3, D5 |
| `slice/fusion.schema.json` | New schema. Closed top-level shape: `version`, `slice`, `generated-at`, `generator`, `requirements[]`. | D4 |
| `adapter.schema.json` | Add optional `cache: opt-out` enum entry. | D8 |

`additionalProperties: false` posture is preserved everywhere it exists today. Every new field is optional; every existing plan, evidence file, candidate block, and adapter manifest validates without change.

### Types

`augentic/specify-cli/crates/domain/` adds:

| Type | Module | Decision |
| --- | --- | --- |
| `ExampleClaim` | `domain/src/evidence/claim/example.rs` | D1 |
| `AuthorityOverrides` (per-Evidence map) | `domain/src/evidence/authority.rs` | D2 |
| `SliceAuthorityOverride` (per-slice map) | `domain/src/change/plan/core/model.rs` | D3 |
| `FusionIndex`, `FusionRequirement`, `FusionResolution` enum | `domain/src/slice/fusion.rs` | D4 |
| `CandidateAliases` | `domain/src/discovery/candidate.rs` | D6 |
| `AutoReviewPrecondition` (closed enum on `Error`) | `domain/src/change/plan/core/create.rs` | D7 |
| `CacheFingerprint`, `CacheIndexEntry` | `domain/src/adapter/cache.rs` | D8 |
| `EventKind::SliceExtractCacheHit`, `::SliceExtractCacheMiss` | `domain/src/journal.rs` | D8 |

All eight follow the standing crate naming, error-variant, and DTO conventions in [`specify-cli/docs/standards/`](https://github.com/augentic/specify-cli/blob/main/docs/standards/style.md).

### CLI surface

Additive verbs and flags only. Existing verbs keep their existing signatures.

| Command | Flags / arguments | Decision |
| --- | --- | --- |
| `specify plan create` | `--auto-review`, `--intent <text>`, `--divergence-likely <slice>`, `--authority-override <slice> <kind>=<key>` | D3, D5, D7 |
| `specify plan amend` | `--divergence likely`, `--authority-override <slice> <kind>=<key>`, `--clear-authority-override <slice> <kind>`, `--clear-authority-overrides <slice>`, `--add-alias <candidate-id>=<alias>`, `--remove-alias <candidate-id>=<alias>` | D3, D5, D6 |
| `specify plan add` | `--sources <key>=<id-or-alias>`, `--authority-override <kind>=<key>` (repeatable) | D3, D6 |
| `specify slice fusion show <slice>` | `--format text|json` | D4 |
| `specify discovery show` | `--aliases` | D6 |
| `specify source resolve` | `--explain <adapter>` | D8 |

Exit-code mapping (per `specify-cli/src/output.rs`):

| Error | Code | Notes |
| --- | --- | --- |
| `slice-authority-override-orphan-source-key` | 2 | `EXIT_VALIDATION_FAILED` |
| `slice-fusion-drift` | 2 | `EXIT_VALIDATION_FAILED` (covers both spec.md ↔ fusion.yaml requirement-id drift and contributing-claim → evidence drift) |
| `discovery-alias-collision` | 2 | `EXIT_VALIDATION_FAILED` |
| `captures-format-invalid` | 1 | `EXIT_GENERIC_FAILURE` (adapter-level I/O error) |

### Writer ownership additions

The RFC-25 writer-ownership table (§Writer ownership) gains three rows. Every cell follows the same "CLI is the single writer" posture as the rest of the table.

| Artifact | Writer |
| --- | --- |
| `fusion.yaml` | `specify slice` * (specifically `slice transition <name> refined`) |
| `cache/.../index.jsonl` | `specify source resolve` and `slice.extract` CLI paths |
| `plan.yaml.slices[].divergence` (any value) | `specify plan create` / `specify plan amend` |

The `plugins/spec/skills/plan/SKILL.md` and `plugins/spec/skills/refine/SKILL.md` change in the same release to invoke these verbs rather than patching files directly.

### Synthesis updates

`/spec:refine`'s synthesis substep order is unchanged (`proposal → specs → design → tasks`). One step is added between `tasks` and `validate`:

5. **Write `fusion.yaml`.** Compute one entry per `REQ-*` id in `spec.md`; record contributing claims, resolution path, and override trace. Atomic write.
6. (was 5) Run `specify slice validate` (now also verifies `spec.md ↔ fusion.yaml` consistency).
7. (was 6) Transition to `refined`.

The synthesis playbook docs under [`plugins/spec/references/synthesis/`](../plugins/spec/references/synthesis/authority.md) gain one new page (`fusion.md`) and amend `authority.md` line 105 to remove the "deferred" note and reference §Authority widening of this RFC.

## Repository layout

```text
/
|-- adapters/sources/
|   |-- captures/                # new (D1)
|   |   |-- adapter.yaml
|   |   `-- briefs/{enumerate,extract}.md
|   `-- (intent, documentation, code-typescript, screenshots — unchanged)
|-- plugins/
|   |-- rt/                          # body retained; skills become thin wrappers (D1)
|   `-- spec/
|       |-- skills/plan/             # SKILL.md step 3 rewritten (D5)
|       |-- skills/refine/           # adds fusion.yaml write step (D4)
|       `-- references/synthesis/
|           |-- authority.md         # amend deferred note (D2, D3)
|           `-- fusion.md            # new (D4)
`-- schemas/                         # (in specify-cli) additive deltas above
```

No file deletions. The RT plugin retains its `wiretapper` skill (TypeScript instrumentation is not a Specify-spine concern); `replay-writer` collapses into the `captures` source adapter's `extract` brief plus the target adapter's build-time fixture-replay hook.

## Implementation plan

Subagent-sized waves, matching the conventions in [rfc-25-plan.md](rfc-25-plan.md). Each chunk lands in one of the two repos (`cli` = `augentic/specify-cli`, `plg` = `augentic/specify`) and carries the acceptance scenario ids it unblocks in brackets.

```text
Wave A (cli)            Schemas + types + CLI verbs                    sequential
   |
Wave B (plg)            Source adapter body + skill rewrites            parallel after A
   |
Wave C (cli + plg)      Build-time fixture replay + acceptance fixtures parallel after B
   |
Wave D (plg)            Docs, AGENTS.md, decision-log, migration note   parallel after C
```

### Wave A — CLI foundation

| Chunk | Repo | Files | Acceptance |
| --- | --- | --- | --- |
| A.1 | cli | All five schema deltas above (D1, D2, D3, D4, D6, D8). | #26-1, #26-3, #26-4 |
| A.2 | cli | Domain types: `ExampleClaim`, `AuthorityOverrides`, `SliceAuthorityOverride`, `FusionIndex`, `CandidateAliases`, `CacheFingerprint`. | #26-1, #26-3 |
| A.3 | cli | `specify plan create --auto-review` (D7); flag stamps `lifecycle: reviewed` in the same atomic write as `plan.create`, emits both `plan.create` and `plan.transition.reviewed` events, valid on any plan shape. | #26-7 |
| A.4 | cli | `specify plan create/amend --divergence likely` (D5); skill-write retired path documented in error message. | #26-5 |
| A.5 | cli | `specify plan create/amend --authority-override` (D3); `specify slice validate` orphan-key check. | #26-3 |
| A.6 | cli | `specify slice fusion show` (D4) + `specify discovery show --aliases` (D6) + `specify source resolve --explain` (D8). | #26-4, #26-6, #26-8 |
| A.7 | cli | Journal events `slice.extract.cache-hit` / `.cache-miss` + `.specify/.cache/.../index.jsonl` writer. | #26-8 |

### Wave B — Adapter and skill bodies

| Chunk | Repo | Files | Acceptance |
| --- | --- | --- | --- |
| B.1 | plg | `adapters/sources/captures/{adapter.yaml,briefs/enumerate.md,briefs/extract.md}`. | #26-1 |
| B.2 | plg | `plugins/spec/skills/plan/SKILL.md` step 3 rewrite (D5); `plugins/spec/skills/refine/SKILL.md` adds the fusion-write step (D4). | #26-4, #26-5 |
| B.3 | plg | `plugins/spec/references/synthesis/fusion.md` (new) + amend `authority.md`. | #26-3, #26-4 |
| B.4 | plg | `plugins/rt/skills/replay-writer/SKILL.md` rewritten as a target-side fixture-runner brief consumed by `adapters/targets/omnia/briefs/build.md`. | #26-1 |

### Wave C — End-to-end acceptance

| Chunk | Repo | Files | Acceptance |
| --- | --- | --- | --- |
| C.1 | plg | `tests/fixtures/sources/captures/` golden tree (one slice's fixtures, expected Evidence). | #26-1 |
| C.2 | cli | Cross-repo acceptance tests for #26-1 … #26-8. | All |
| C.3 | plg | `adapters/targets/omnia/briefs/build.md` amendment that opts into the (optional) fixture-replay hook for Omnia; matching fixture under `tests/fixtures/targets/omnia/` that exercises a target which omits the hook (`fixture-replay` field absent) and one that implements it. | #26-1 |

### Wave D — Docs

| Chunk | Repo | Files |
| --- | --- | --- |
| D.1 | plg | `AGENTS.md` adds two paragraphs on the new vocabulary (`captures`, `fusion.yaml`, per-slice `authority-override`). |
| D.2 | plg | `.cursor/rules/project.mdc` adds rows to the source-adapter list. |
| D.3 | plg | `docs/migration/2.1.md` documents the additive shape and the (no-op) upgrade path. |
| D.4 | cli | `DECISIONS.md` adds rows for: per-kind authority on Evidence, per-slice authority on `plan.yaml`, `fusion.yaml` as audit-only artifact, cache fingerprint inputs. |

The release tag is `2.1.0` in both repos. There is no `migrate-to-2.1.sh` — every change in this RFC is additive.

## Acceptance scenarios

Per-RFC numbering: scenarios are prefixed `#26-N` to disambiguate from RFC-25's `#1`–`#12` series. Every scenario maps to at least one normative decision.

| #     | Decisions | Scenario | What it stress-tests |
| ----- | --------- | -------- | -------------------- |
| #26-1 | D1 | **Runtime source binding end-to-end.** Operator binds `runtime=./captures/replays` alongside `legacy=./vendor/monolith`. | `enumerate` walks the capture tree; `extract` emits `kind: example` claims with `fixture-digest`; `Sources: [legacy-monolith, runtime]` on a synthesis-resolved `Status: agreed` block; the Omnia target's `build` runs captures through generated code and writes `fixture-replay: { passed, failed, skipped, ran-at, runner }` into `.metadata.yaml`; `merge` surfaces the summary in its closing message; operator policy or a forked target adapter (not core) gates whether `failed > 0` blocks landing. |
| #26-2 | D1, D2 | **Behaviour-class override on a `criterion` claim.** Combined evidence where docs say "30 minutes" expiry and runtime captures show 24-hour expiry; the operator wants runtime to win. | Per-slice `authority-override: { criterion: runtime }` lands via `plan amend`; synthesis writes `Status: divergence` with runtime as the operative value and docs preserved as commentary; `fusion.yaml.requirements[].resolution-trace.step` reads `per-slice-authority-override`. |
| #26-3 | D2, D3 | **Per-Evidence override with no per-slice override.** Adapter emits `authority-overrides: { decision: documentation }`; no per-slice override. | Synthesis resolves `decision`-class disagreement via per-Evidence override; `requirement`-class falls back to RFC-25 default ordering; `fusion.yaml` records both resolution paths. |
| #26-4 | D4 | **`fusion.yaml` round-trip.** `/spec:refine` writes the index with inline `value` payloads on every `contributing-claim`; `specify slice fusion show <slice>` prints both winning and dropped values without opening `evidence/*.yaml`; operator hand-edits `spec.md` to flip a `[divergence]` to `[agreed]`; `specify slice validate` reports `slice-fusion-drift`. | Validate detects requirement-id drift AND contributing-claim → evidence drift; operator re-runs `/spec:refine` to regenerate; drift clears; lifecycle reaches `refined`. |
| #26-5 | D5 | **CLI-only `divergence: likely`.** Plan skill invokes `specify plan amend --divergence likely`; the skill no longer reads or writes `plan.yaml` directly. | `plan.propose.divergence` journal event fires once from the CLI; no skill-side YAML mutation; the file diff is byte-identical to the pre-D5 skill-written output. |
| #26-6 | D6 | **Cross-source candidate alias.** Docs surface `account-pwd-reset`; code surfaces `password-reset`; operator adds an alias via `plan amend --add-alias`. | `specify plan add --sources legacy=password-reset` rewrites the value to the resolved canonical `id` before persisting; downstream `extract` runs once per source against the resolved candidate; re-enumeration preserves operator-added aliases. |
| #26-7 | D7 | **Auto-review at create across plan shapes.** Operator runs `--auto-review` against (a) a single-slice pure-intent plan, (b) a single-slice path-bound plan, and (c) a hand-authored multi-slice plan with two sources. | All three plans land at `lifecycle: reviewed` in one CLI call; `plan.transition.reviewed` journal event fires once per plan; `/spec:execute` accepts each plan immediately; `plan validate` failures (e.g. orphan source key) refuse the create regardless of `--auto-review`; running the explicit `specify plan transition <name> reviewed` after `--auto-review` is a no-op. |
| #26-8 | D8 | **Cache fingerprint hit vs miss.** Two consecutive `/spec:refine` runs on the same slice; between them, the operator bumps the adapter version. | First run emits `slice.extract.cache-miss` with `reason: no-prior-entry`; second run (no input change) emits `slice.extract.cache-hit`; third run after version bump emits `slice.extract.cache-miss` with `reason: adapter-version-changed`; the `index.jsonl` log carries one row per write. |

Scenarios #26-1 and #26-2 are the release blockers: D1 is the largest substantive change, and D2 + D3 are the public face of the authority widening that the synthesis review identified as the highest-frequency operator pain.

## Migration

RFC-27 is **strictly additive**. There is no `migrate-to-2.1.sh`. Concrete consequences:

**For operators.**
- Existing `plan.yaml`, `evidence/*.yaml`, `discovery.md`, and slice directories validate without change.
- Existing `/spec:plan` → `reviewed` → `/spec:execute` → `/spec:finalize` rhythm is unchanged.
- New verbs and flags are opt-in. An operator who never runs `--auto-review`, never adds a per-slice `authority-override`, never binds a `captures` source, and never opens `fusion.yaml` sees no behavioural change.

**For source adapter authors.**
- `captures` ships as a new first-party adapter; existing adapter manifests are unaffected.
- The optional `authority-overrides` map on Evidence files is new and unused by existing adapters; emitting it is opt-in.
- The optional `aliases[]` field on candidate blocks is new and unused by existing adapters; emitting it is opt-in. `propose` may populate it on uncertain merges.
- The optional `cache: opt-out` flag on `adapter.yaml` is opt-in; the default behaviour for every existing adapter is fingerprint-on and cache-on.

**For target adapter authors.**
- The build-time fixture-replay hook is **optional in v1**. Targets that never want to consume captures (Vectis today) need do nothing; targets that do (Omnia, contracts) add a fixture-runner step to their `build` brief and emit `fixture-replay: { passed, failed, skipped, ran-at, runner }` into `.metadata.yaml`. Targets that have not implemented the hook simply omit the field; `merge` does not require it.
- `merge` is advisory on replay results in v1 — it surfaces the summary in its closing message but does not auto-refuse on `failed > 0`. Targets or operators who want strict gating wire it themselves (custom target adapter or CI policy reading `specify slice outcome show <slice> --format json`).
- `shape` is unchanged.

**For skill authors consuming planning artifacts.**
- `plan.yaml.slices[].authority-override` is a new optional field; consumers ignoring it preserve existing behaviour.
- `fusion.yaml` is a new artifact; consumers ignoring it preserve existing behaviour. Consumers wanting machine-readable resolution traces use `specify slice fusion show <slice> --format json`.

There is **no breaking change** to: existing schemas (all deltas are additive), existing CLI verbs (new verbs and new flags only), existing exit codes (new error discriminants stay within the existing exit-code mapping), existing journal events (`slice.extract.cache-hit` / `.cache-miss` are new event kinds, not changes to existing payloads), or any existing adapter manifest.

## Alternatives considered

### A1 — Per-claim authority (rejected)

The synthesis review noted that per-claim-kind authority is "the right move at least half the time on legacy work" and explicitly left per-claim authority for a future RFC. D2 implements per-kind on Evidence and D3 implements per-slice override; per-claim stays deferred. Reason: per-claim authority forces a `claim.authority:` field that competes with the document-level `authority:` and reduces the operator's mental model from "this adapter is authoritative for these kinds" to "this individual sentence is authoritative." The reduction is finer-grained but is also a meaningfully larger schema and review surface. The override seam for the rare per-claim case stays as today: hand-edit `spec.md` after `/spec:refine`.

### A2 — Graph-of-claims persistence (rejected; in synthesis as Option D)

The synthesis review classified the graph-of-claims direction as "right instinct, wrong execution path." D4 implements the thin reconciliation index — every `REQ-*` id with its contributing claims and resolution outcome — without introducing a graph schema, a graph validator, or a graph database. Operators get the inspectable artifact without the engineering cost; if a future RFC demands the full graph, `fusion.yaml` is the natural source of truth to lift from.

### A3 — Iterative critique loop as the default (rejected; in synthesis as Option B)

Same posture as the synthesis review: keep it as an opt-in breakout flag on `/spec:refine`. D1–D8 do not add a critique loop. The fixture-replay hook in D1 already provides a deterministic outer-loop check (generated code runs against captured I/O) without requiring an iterative inner loop inside synthesis.

### A4 — Multi-class authority widening (e.g. `regulation > intent > documentation > behaviour`) (rejected for v1)

Adding new authority classes would force a schema change and disturb every consumer that parses the closed enum. The per-slice override (D3) covers the practical use cases (compatibility-driven migrations, regulated domains where docs outrank code, fixtures-as-truth) with a one-line plan edit rather than a new ontology. Reinstate if and when a real consumer asks.

### A5 — `--auto-review` only for single-slice pure-intent plans (rejected)

[rfc-25-synthesis.md](rfc-25-synthesis.md) §4 recommended narrowing `--auto-review` to single-slice pure-intent plans. RFC-27 broadens the flag to any plan shape after weighing the alternative: the operator's review is the act of typing the create command, and that act is the same whether the plan has one slice or ten. Forcing a second CLI invocation on hand-authored multi-slice plans buys no review value — the operator already named every binding on the same line — and pushes the most-experienced operators toward muscle-memorising the transition command anyway, eroding rather than reinforcing the Gate-1 trust seam. The flag is opt-in; operators who want agent-led `propose` review continue to use the two-call path and see byte-identical final state.

### A6 — Fixture replay as a required `build` step with auto-`merge`-refusal (rejected)

An earlier draft of D1 made the fixture-replay hook a hard `build` step and refused `merge` on `failed > 0`. RFC-27 keeps the hook target-optional and operator-visible after weighing two costs of the strict posture: (1) every target adapter would need to implement the hook before v1 lands, blocking the release on Vectis and contracts work that has no `captures` consumer; and (2) auto-refusal at `merge` makes synthesis-tag posture inconsistent — RFC-25 explicitly leaves `[conflict]` and `[divergence]` as review signals rather than gates, and bolting an automatic gate onto fixture failure breaks that invariant. The optional posture matches RFC-24 §`surfaces[]` (target-specific structured outputs are recorded for operator review, not gated on). A future RFC may promote auto-refusal into core if v1 telemetry shows operators consistently want it.

### A7 — Per-Evidence `priority:` number instead of `authority-overrides` map (rejected)

A numeric priority field would generalise authority resolution but loses the closed-enum guarantee. The synthesis review's diagnosis was specifically about the *closed* nature of the enum being too coarse, not about wanting an unbounded numeric scale. D2 keeps the closed enum and lifts it from per-document to per-(document, kind), which is the smallest change that addresses the diagnosis.

### A8 — `fusion.yaml` as references-only (rejected)

An earlier draft of D4 kept `fusion.yaml` at `(source, claim-id)` references and required operators to open every contributing `evidence/*.yaml` to read the dropped values during a `[divergence]` review. RFC-27 carries inline `value` payloads on every `contributing-claim` after weighing the trade-off: synthesis surprises are rare but high-cost when they happen, and the dominant audit pattern is "what did each source actually say?" — a question that requires opening N evidence files in the references-only design and zero in the inline design. The inline payload adds bounded size to `fusion.yaml` (single-line `value` strings, 16 KiB cap with truncation indicator), preserves byte-stable diffs, and keeps the index a single-file audit surface. The full per-kind body (e.g. `example` claim `input` / `output` blocks) stays in the source evidence file, linked by `path` — `fusion.yaml` is still an index, not a re-encoding.

## Non-goals

- Per-claim authority overrides. D2 stops at per-kind on the Evidence document.
- A new authority class. The closed `intent > documentation > behaviour` enum stays.
- Auto-resolution of `[conflict]` or `[divergence]` tags. Tags remain operator review signals, same as RFC-25.
- A `refined-but-blocked-build` slice lifecycle state. Synthesis tags continue not to gate `build`; if an operator wants a tagged spec to block, they hand-edit and re-validate.
- Idempotent `/spec:refine` against operator hand-edits. The hand-edit-vs-re-refine semantics from RFC-25 are unchanged; `fusion.yaml` is regenerated whole on each `/spec:refine` and does not preserve hand-edits.
- Cross-change reconciliation (e.g. `fusion.yaml` across multiple `change.md` runs). Reconciliation index is per-slice, archived with the slice.
- Hosted execution of the cache index (RM-22 territory).
- Catalog-backed alias resolution (a future RFC if `aliases[]` proves to be load-bearing for catalog imports).
- Replacing the RT plugin's TypeScript wiretapper. Source-side instrumentation is out of Specify's scope; D1 only promotes the *capture consumption* side into the source-adapter contract.
- Multi-target fixture replay. A slice with two targets is already out of scope per RFC-25 §Non-goals; D1 inherits that posture.

## Open questions

1. **Should per-slice `authority-override` accept a wildcard kind** (`authority-override: { "*": runtime }`)? Current preference: no — the per-kind ergonomic benefit comes from operators thinking about which kinds matter; a wildcard would re-collapse the surface.
2. **Should `captures` enumerate emit a `kind: example-set` candidate** (one block per handler) or a `kind: example` block per individual fixture? Current preference: per-handler (matches the slice grain operators reason about); the per-fixture detail lives in `extract`-time claims.
3. **Should the fixture-replay hook eventually become a separate `validate` capability on target adapters** rather than living inside `build`? Current preference: keep it inside `build` for v1 (three capabilities — `shape`, `build`, `merge` — is the simplest model that works; a `validate` capability would compete with the existing `specify slice validate` verb). Revisit if telemetry shows targets routinely want to run replay independently of the build step.
4. **Should `cache: opt-out` adapters emit a warning** on every cache-bypassed run? Current preference: no — opt-out is a deliberate choice and the journal `reason: adapter-opt-out` carries the audit trail without operator-visible noise.
5. **Should the per-slice `authority-override` resolution trace appear in `spec.md`** (as commentary on `[divergence]` blocks), or live only in `fusion.yaml`? Current preference: `fusion.yaml` only — `spec.md` stays operator-facing behavioural prose; the audit trace stays in the index.
6. **Should v1.5 promote `merge` auto-refusal on `fixture-replay.failed > 0` into core** behind an opt-in `policy` field on the slice? D1's optional-and-advisory posture matches RFC-25's tag-and-proceed invariant; promotion is the cleanest place to put a future strict-mode gate without bolting it onto the v1 surface. Defer until v1 telemetry shows operator demand.
7. **Should `--auto-review` accept `--intent @<path>`** to read the intent body from a file rather than the CLI argument, now that the flag spans more than the degenerate N=1 case? Current preference: yes, and the same `@<path>` convention should apply to any future flag that takes free-form text. Lift in a follow-up if the implementation cost is non-trivial.

## Observability ([RFC-19](rfc-19-observability.md))

Additive events. None replace or modify existing event payloads.

| Event | When |
| --- | --- |
| `slice.extract.cache-hit` | Cache lookup matched; `extract` was not re-run (D8) |
| `slice.extract.cache-miss` | Cache lookup missed; `extract` ran. Payload carries `reason` (D8) |
| `slice.fusion.written` | `/spec:refine` wrote `fusion.yaml` (D4) |
| `slice.fixture-replay.completed` | Target's `build` finished fixture replay (optional hook; absent for targets that did not implement it). Payload carries `{ passed, failed, skipped, runner }` (D1) |
| `plan.amend.authority-override` | Operator set / cleared a per-slice override (D3) |

The existing `plan.propose.divergence`, `plan.amend.divergence`, `plan.transition.reviewed`, `slice.transition.refined`, `slice.extract.completed`, and `slice.synthesis.{conflict,divergence,unknown}` events are unchanged.

## References

- [rfc-25-workflow.md](rfc-25-workflow.md) — workflow spine and synthesis contract this RFC sharpens.
- [rfc-25-synthesis.md](rfc-25-synthesis.md) — review note this RFC lifts into normative decisions.
- [rfc-25-plan.md](rfc-25-plan.md) — wave-decomposition format mirrored here.
- [RFC-19](rfc-19-observability.md) — journal events. New events listed in §Observability.
- [RFC-21](rfc-21-catalogue.md) — `sources.yaml`; unaffected by D1 since `captures` is a normal source.
- [RFC-22](rfc-22-ledger.md) — audit-only-field precedent (`mapping`) followed by `fusion.yaml`.
- [RFC-24](rfc-24-omnia.md) — `shape` capability unchanged; `surfaces[]` precedent for additive plan fields.
- [`plugins/spec/references/synthesis/authority.md`](../plugins/spec/references/synthesis/authority.md) — current authority hierarchy; amended by D2.
- [`plugins/rt/skills/replay-writer/SKILL.md`](../plugins/rt/skills/replay-writer/SKILL.md) — body lifts into `captures` and the target-side fixture-runner hook (D1).
- [`plugins/rt/skills/replay-writer/references/capture-format.md`](../plugins/rt/skills/replay-writer/references/capture-format.md) — fixture layout `captures` consumes (D1).
- [`schemas/evidence.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/evidence.schema.json) — extended by D1, D2.
- [`schemas/discovery/candidate.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/discovery/candidate.schema.json) — extended by D6.
- [`schemas/plan/plan.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/plan/plan.schema.json) — extended by D3, D5.
- [`crates/domain/src/journal.rs`](https://github.com/augentic/specify-cli/blob/main/crates/domain/src/journal.rs) — extended by D8.
- [`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) — gains four rows per D.4 of the implementation plan.
- [roadmap.md](roadmap.md) — D8's cache-fingerprint events feed RM-14's structured workflow events.
