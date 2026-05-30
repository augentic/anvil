# RFC-29a M1 — C0 Preflight Decisions

> **Temporary scaffolding.** Single source of truth for the C1–C9 subagents of the RFC-29a M1 implementation plan. Delete at end of milestone. All shapes below are pinned against the real code, with `file:line` citations into `augentic/specify-cli` (`specify-cli/…`) and `augentic/specify` (`specify/…`). No production code was changed by C0.

## 1. `$SCRATCH_DIR` path shape

The CLI has **two disjoint cache roots** under `.specify/.cache/`, each its own subtree (no co-tenancy):

- **Manifest cache** — `.specify/.cache/manifests/{sources,targets}/<name>/` (`MANIFESTS_CACHE_DIR`, `specify-cli/crates/workflow/src/adapter/core.rs:46`; `cache_dir()` at `core.rs:196`).
- **Extraction result cache** — `.specify/.cache/extractions/<adapter>/…` (`EXTRACTIONS_CACHE_DIR = "extractions"`, `specify-cli/crates/workflow/src/adapter/core.rs:58`).

The extraction **result cache** occupies exactly these children of `.specify/.cache/extractions/<adapter>/` (all via `CacheLayout`, `specify-cli/crates/workflow/src/adapter/cache/io.rs`):

| Artifact | Path | Citation |
| --- | --- | --- |
| adapter root | `extractions/<adapter>/` | `io.rs:62` (`adapter_dir`) |
| per-fingerprint dir | `extractions/<adapter>/<hex-digest>/` | `io.rs:71` (`fingerprint_dir`) |
| cached artifact | `extractions/<adapter>/<hex-digest>/<evidence.yaml \| lead-set.md>` | `io.rs:84` (`artifact_path`) |
| fingerprint record | `extractions/<adapter>/<hex-digest>/fingerprint.json` | `io.rs:78` (`fingerprint_record_path`) |
| append-only index | `extractions/<adapter>/index.jsonl` | `io.rs:90` (`index_path`) |

`<hex-digest>` is the **64-char lowercase sha256 hex** with the `sha256:` prefix stripped (`digest_dir_name`, `io.rs:325`).

### Pinned `$SCRATCH_DIR` (C5/C6/C7 build these; not yet in code)

```text
extract → .specify/.cache/extractions/<adapter>/<slice>/scratch/
survey  → .specify/.cache/extractions/<adapter>/survey/scratch/
```

`<adapter>` is the resolved source-adapter `name`; `<slice>` is the slice name passed to `extract --slice`.

**Disjointness proof.** The only result-cache children of `extractions/<adapter>/` are (a) 64-char-hex `<digest>/` directories and (b) the file `index.jsonl`. Scratch nests one level deeper under a `<slice>/` or literal `survey/` segment, then a `scratch/` leaf. A 64-char-hex digest dir cannot equal a kebab slice name (kebab contains `-` and is short; hex is exactly 64 chars of `[0-9a-f]`), and `survey` is not a valid hex string. So no scratch path can ever land on a result-cache artifact path. ✔

**Two collision edge cases C6/C7 must guard (flagged, not blocking):**
- A slice named literally `survey` makes the extract scratch `extractions/<adapter>/survey/scratch/` **identical** to the survey-op scratch. Recommend C6/C7 reserve `survey` as a non-slice segment or key extract scratch under a `slices/` infix if this is a concern.
- A (pathological) 64-hex-char slice name could shadow a digest dir's *parent namespace*. Not reachable through normal slice-name grammar, but worth a one-line guard.

## 2. Agent prepare/finalize handoff envelope

The stdout JSON the `prepare` phase prints. **Field names are kebab-case on the wire** (serde `rename_all = "kebab-case"`), matching the established `PreviewBody` (`specify-cli/src/runtime/commands/source/preview.rs:31`) and `ResolveBody` (`specify-cli/src/runtime/commands.rs:193`) conventions — i.e. Rust `scratch_dir` → wire `scratch-dir`.

Full field set:

```json
{
  "adapter": "documentation",
  "version": 1,
  "briefs-dir": "/abs/.../adapters/sources/documentation/briefs",
  "source-dir": "/abs/path/to/bound/source",
  "scratch-dir": "/abs/.../.specify/.cache/extractions/documentation/<slice|survey>/scratch",
  "evidence-dir": "/abs/.../evidence",
  "leads": ["lead-a", "lead-b"],
  "execution": "agent"
}
```

### Per-operation presence matrix (DECISION — C6/C7 conform)

| Field | `survey` (prepare) | `extract` (prepare) | Notes |
| --- | --- | --- | --- |
| `adapter` | ✔ | ✔ | resolved `manifest.name` |
| `version` | ✔ | ✔ | resolved `manifest.version` (`u32`) |
| `briefs-dir` | ✔ | ✔ | absolute path to the adapter's `briefs/` dir (from C1 `briefs-dir` on the resolve envelope) |
| `source-dir` | path bindings only | path bindings only | **absent** for value-bound sources (`intent`); serialise with `skip_serializing_if = "Option::is_none"` |
| `scratch-dir` | ✔ (`…/survey/scratch`) | ✔ (`…/<slice>/scratch`) | §1 |
| `evidence-dir` | **absent** | ✔ | `survey` writes a lead set, not Evidence. The survey output target is the surveyed lead set merged via `Discovery::merge_survey` (C4); `extract` writes `evidence/<source-key>.yaml` |
| `leads` | ✔ (populated — the surveyed candidates / target lead set) | single element (the one `<lead-id>` being extracted) | `Vec<String>`; for `extract` carry exactly the one positional `<lead-id>` |
| `execution` | constant `"agent"` | constant `"agent"` | literal; the `tool` path is single-phase and never prints this envelope |

Conventions to copy verbatim:
- `kebab-case` rename on the body struct (`preview.rs:32`, `commands.rs:194`).
- Path fields are `PathBuf` (absolute) — `preview.rs` uses `PathBuf` for `source`/`out`/`evidence_dir`.
- `leads` elides when empty in `preview` (`#[serde(skip_serializing_if = "Vec::is_empty")]`, `preview.rs:39`); for the handoff envelope `survey` **always** populates it, `extract` carries the single lead.
- Optional fields (`source-dir`, `evidence-dir`) use `Option<PathBuf>` + `skip_serializing_if = "Option::is_none"`.

> **Value-binding (C7).** For value-bound `intent`, `source-dir` is absent and the source request carries `value-inline: <string>`; path bindings carry `source-path`. This maps onto the existing `FingerprintSource::{Path, Value}` (`specify-cli/crates/workflow/src/adapter/cache.rs:138`); `FingerprintSource::from_value` hashes the literal body (`cache.rs:177`), `from_path` canonicalises (`cache.rs:166`). Do **not** introduce a new cache machinery or the RFC-29d build-request schema.

## 3. M1 journal payload variants

Modelled exactly on the existing `EventKind` variants in `specify-cli/crates/workflow/src/journal.rs` — adjacently tagged `{ "event": <id>, "payload": {…} }` (`journal.rs:59-61`), `#[serde(rename = "<wire-id>", rename_all = "kebab-case")]` per variant, Rust fields `snake_case`. Template to copy: `SliceExtractCacheHit` (`journal.rs:170`) / `SliceExtractCacheMiss` (`journal.rs:185`).

C3 adds these three first-class typed variants:

```rust
/// wire id: source.survey.cache-hit
#[serde(rename = "source.survey.cache-hit", rename_all = "kebab-case")]
SourceSurveyCacheHit {
    source_key: String,   // wire: source-key  (from plan.yaml.sources.<key>)
    adapter: String,      // wire: adapter     (adapter.yaml.name, kebab)
    fingerprint: String,  // wire: fingerprint (sha256:<hex> of CacheFingerprint inputs)
},

/// wire id: source.survey.cache-miss
#[serde(rename = "source.survey.cache-miss", rename_all = "kebab-case")]
SourceSurveyCacheMiss {
    source_key: String,        // wire: source-key
    adapter: String,           // wire: adapter
    fingerprint: String,       // wire: fingerprint
    reason: CacheMissReason,   // wire: reason  (REUSE existing enum — see below)
},

/// wire id: source.execution.agent
#[serde(rename = "source.execution.agent", rename_all = "kebab-case")]
SourceExecutionAgent {
    source_key: String,        // wire: source-key
    adapter: String,           // wire: adapter
    operation: SourceOperation,// wire: operation  ("survey" | "extract")
},
```

- `operation` should be the existing closed `SourceOperation` enum (`survey | extract`), re-exported via `crate::adapter::operation::SourceOperation` (`specify-cli/crates/workflow/src/adapter/cache.rs:234`). It already serialises kebab (`"survey"` / `"extract"`), confirmed by `cache.rs:302`.
- C3 must extend `event_wire_shapes_match_contract` and `no_snake_case_leaks_to_wire` (`journal.rs:565`, `journal.rs:726`) to cover the three new variants.

### `CacheMissReason` — REUSE, do not re-add

The closed enum already exists and **already carries `AdapterOptOut`** — confirmed at `specify-cli/crates/workflow/src/journal.rs:347` (variant `AdapterOptOut`, wire `adapter-opt-out`). Full closed set (`journal.rs:334-348`):

| Rust variant | Wire spelling |
| --- | --- |
| `NoPriorEntry` | `no-prior-entry` |
| `SourcePathChanged` | `source-path-changed` |
| `AdapterVersionChanged` | `adapter-version-changed` |
| `BriefShaChanged` | `brief-sha-changed` |
| `ToolVersionChanged` | `tool-version-changed` |
| `AdapterOptOut` | `adapter-opt-out` |

Round-trip pinned at `journal.rs:654` (`cache_miss_reason_round_trips`). The forced-opt-out survey miss (item 4) emits `reason: adapter-opt-out`. **C3's "add if absent" is a no-op.**

## 4. Cache-posture audit (five first-party source adapters)

Read from `specify/adapters/sources/<name>/adapter.yaml`:

| Adapter | Declares `cache:`? | Effective cache today | `tools:`? |
| --- | --- | --- | --- |
| `intent` | **no** (absent) | **active** (lookups can hit) | no |
| `documentation` | **no** (absent) | **active** | no |
| `code-typescript` | **no** (absent) | **active** | no |
| `screenshots` | **no** (absent) | **active** | no |
| `captures` | **no** (absent) | **active** | yes (`replay-index@0.1.0`) |

How "active" is derived: `SourceAdapter.cache: Option<CacheMode>` defaults to `None` when the field is absent (`specify-cli/crates/workflow/src/adapter/core.rs:237`; test `source_cache_field_defaults_to_none`, `core.rs:624`). `lookup()` only short-circuits to a miss when `cache_mode == Some(CacheMode::OptOut)` (`specify-cli/crates/workflow/src/adapter/cache/io.rs:180`); with `None` it probes the filesystem and **can return `Hit`** (`io.rs:191`).

### Behaviour change under C2's forced opt-out — DECISION

`execution: agent` forces `cache: opt-out` (RFC-29a D9). Therefore:

- **All five adapters change behaviour**: they move from *cache-active* (potential hits) to *forced opt-out* — **guaranteed misses with `reason: adapter-opt-out`** on every `survey`/`extract`. None relies on a declared opt-out today; all five rely on the active default, so the cache hit they could get today disappears under `agent`. **C2 must call this out in its PR** (per RFC-29a §"First-party adapters in M1" and plan §C2 "Callout").
- **None triggers `adapter-execution-agent-cache-conflict`.** That parse check fires only when a manifest **declares** a `cache:` *other than* the forced opt-out. None of the five declare `cache:` at all, so stamping `execution: agent` is a clean schema/loader pass for every one — the change is purely runtime (active → forced opt-out), not a rejection.

## Surprises / discrepancies vs the plan's assumptions

1. **The conflict check is effectively un-triggerable by any legal manifest today.** `CacheMode` is a closed single-variant enum (`OptOut` only — `core.rs:136`), and `source.schema.json#/properties/cache` enumerates only `["opt-out"]` (`specify-cli/schemas/source.schema.json:46`). So there is **no "non-opt-out cache mode" a manifest can declare**. The plan's framing ("would trigger `adapter-execution-agent-cache-conflict` if they declare a non-opt-out cache mode") is correct in principle but vacuous in practice: C2's conflict check guards a *future-widened* `CacheMode`, not anything reachable now. For all five M1 sources the answer is: **no conflict, behaviour change only.**
2. **`execution` added to a manifest today fails schema validation.** `source.schema.json` is `additionalProperties: false` with `execution` absent (`schemas/source.schema.json:7`, properties end `:50`). C2's ordering (add enum to schema *without* `required` → stamp manifests → flip `required`) is therefore mandatory, not optional — stamping before the schema lands would throw `adapter-schema-violation`.
3. **`source.schema.json` already requires `description`** (`schemas/source.schema.json:8`). All five manifests carry it. No action, but C2's example-`adapter.yaml` doc sweep must keep `description` present alongside the new `execution`.
4. **`captures` declares a tool** (`replay-index@0.1.0`); its `tool_versions` feed the fingerprint. Irrelevant under forced opt-out (lookups never run), but C6/C7 should still populate `tool_versions` for the audit `index.jsonl` row, which `write()` appends even on opt-out (`io.rs:264`, test `adapter_opt_out_misses` at `io.rs:404`).
5. **No `briefs-dir` exists on the resolve envelope yet** — confirmed: `ResolveBody` has only `axis, name, resolved-path, location, operations, description` (`specify-cli/src/runtime/commands.rs:193-202`). C1's premise holds. The handoff `briefs-dir` (§2) is `<resolved-path>/briefs` since brief paths in the manifest are relative (e.g. `briefs/extract.md`) joined onto the adapter root (`preview.rs:68`).
6. **`survey` has no `evidence-dir`.** The plan's envelope lists `evidence-dir` flatly; pinned decision (§2) is that `evidence-dir` is **absent for `survey`** (it produces a lead set merged via `Discovery::merge_survey`, not an Evidence file). C6 should not scaffold `evidence/` for survey.
