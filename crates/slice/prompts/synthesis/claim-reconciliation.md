# Claim reconciliation

How the agent groups claims across `Evidence[]` into the synthesis response and where each claim kind lands in the four artifacts. Grouping and the `agreement` verdict are the agent's; the kernel resolves authority, derives `status`, marks winners, and renders the `Sources:` list.

## Per-kind reconciliation

Closed `kind` enum → landing surface and reconciliation key:

| Kind | Authority class | Lands in | Key |
| ---- | --------------- | -------- | --- |
| `requirement` | `documentation` | `specs/<domain>/spec.md` — one block per `id` group | `id` (required) |
| `criterion` | `documentation` | same spec block as matching `<requirement>.*` prefix → `#### Scenario:`; else nearest requirement by source order | `id` (required) |
| `decision` | `documentation` | `design.md` under the H2 it informs; quote `(from <source>)` | none |
| `section` | `documentation` | `design.md` relevant H2; or `proposal.md` `## Why` when it names the *why* | none |
| `excerpt` | `behaviour` | primarily `design.md` `## Technical logic`; alone → `spec.md` requirement; vs `documentation` → `[divergence]` commentary | optional `id` / handler from `path` |
| `type` | `behaviour` | `design.md` `## Domain model` — `signature` verbatim | optional `id` / type name |
| `call` | `behaviour` | `design.md` `## APIs and integrations` or `## Technical logic` | optional `id` / `callee` |
| `example` | `behaviour` | `spec.md` via matching `id` prefix (else own requirement); `design.md` references capture `path` | required `id` |
| `region` / `container` / `leaf` | `documentation` (spatial) | `design.md` `## UI / layout` tree; never `spec.md` requirements | none (positional) |
| `intent` | `intent` | `proposal.md` `## Why`; optional headline `spec.md` requirement when it names a behaviour | none (one per Evidence) |
| `diagram` | source-dependent | `design.md` relevant H2 | none |
| `contract` | source-dependent | `design.md` `## APIs and integrations` | none |

### Deterministic reconciliation on `id`

`requirement` and `criterion` claims MUST carry `id`. Group every contributing claim by exact `id` across all Evidence — that is the cross-source key.

- All claims sharing one `id` collapse into one requirement, carrying every contributing `(source, id, kind)` claim.
- The kernel renders `Sources:` from those claims (highest authority first) and derives `status` from claim count, `agreement`, and resolved authority (see [`authority.md`](authority.md)).
- Matching `statement:` / `criterion:` (after trivial whitespace normalisation) → `agreement: agreed`. Disagreeing values → `agreement: disagreed`; the kernel applies per-authority resolution.

### Behaviour claims as corroboration

`excerpt`, `type`, `call`, and `example` (default class `behaviour`) primarily drive `design.md`. They contribute to `spec.md` when:

- **Standalone** — no other source supplied a `requirement` on the same surface → one requirement with that single claim (`Status: agreed`).
- **Authority-loser** — a `documentation` `requirement` contradicts an `excerpt` / `example` → `agreement: disagreed`; documentation wins by default; behaviour survives as the `Note:` on the `[divergence]` block (see [`authority.md`](authority.md)). Flip per slice via `authority-override`.

`example` claims (from `captures`) are siblings of `excerpt` / `call` at the same class — no silent preference. Tie-break with `authority-override.<kind>` ([§Per-slice overrides](authority.md#per-slice-overrides-on-planyaml)). A captures Evidence MAY emit `authority-overrides: { example: documentation }` to lift examples above document-level `behaviour` (rare). Body fields (`id`, `path`, `replay-digest`, `input`, `output`, optional `statement`) are owned by the captures extract prompt — refer there; do not mirror them here.

### Spatial and intent

- Spatial (`region` / `container` / `leaf`) → one `design.md` `## UI / layout` tree per screen. Vectis `build` reads it for `composition.yaml`; other targets omit the H2. Behavioural assertions that *use* layout stay as separate `requirement` claims.
- `intent` (exactly one claim per Evidence): render `statement` as `proposal.md` `## Why`; if it names a behaviour, also one headline requirement. Pure-intent slices get at most one requirement unless other sources contribute.

## Per-authority resolution (slice-time)

When a `disagreed` `id` group has multiple authorities, the kernel's [§Resolution order](authority.md#resolution-order) picks the winner — that reference is canonical for `intent > documentation > behaviour` and overrides. Landing rules:

- **Strict-greater authority → `Status: divergence`.** Docs "30 minutes" vs code "15 minutes" → body carries 30; `Note:` preserves 15. A per-slice override flips body/`Note:` without changing the `divergence` posture.
- **Tied authority → `Status: conflict`.** Same-class disagreement is `[conflict]` unless a per-slice override breaks the tie.
- **Agreement (including after override alignment) → `Status: agreed`.**

## Order and stability

- Kernel `Sources:` sort: authority class (`intent` < `documentation` < `behaviour`), then alphabetically by key within a class; highest-authority key first.
- Order requirements by source order on the highest-authority Evidence (tie → alphabetical on first contributing key); the kernel assigns `REQ` ids in that declaration order.
- Re-running the refine phase on identical `Evidence[]` and `shape` MUST produce byte-identical artifacts.

## Plan-time reconciliation is a separate playbook

Plan-time `Lead[]` reconciliation inside `emery plan author` is documented in the plan CLI reference. Cross-source matching is agent judgment; the kernel validates partition shape only. The operator curates during plan review via `change.md` and `emery plan amend`.
