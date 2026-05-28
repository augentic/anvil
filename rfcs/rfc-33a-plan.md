# RFC-33a Implementation Plan

Companion to [`rfc-33a-ignore-directives.md`](rfc-33a-ignore-directives.md). Decomposes the RFC's seven `## Implementation plan` steps into context-bounded changes sized for individual subagents, with explicit dependencies and parallelism markers.

## Repo split

| Repo | Scope |
| --- | --- |
| `augentic/specify-cli` | Schemas, DTOs, indexer extractor, scanner pipeline, journal variant, exit semantics, goldens |
| `augentic/specify` | Two `UNI-*` markdown rule files under `adapters/shared/rules/universal/` |

Every change below names the repo it lands in. Cross-repo touches are called out explicitly so the orchestrating agent can decide whether to land them in one PR (parent repo first) or as separated PRs.

## Wave overview

```text
Wave 1 (3 parallel)   →  Wave 2 (2 parallel)  →  Wave 3 (1)  →  Wave 4 (2 parallel)
  C1 schema                 C4 finding DTOs        C6 pass        C7 journal wiring
  C2 rules markdown         C5 model + indexer                    C8 goldens + accept
  C3 journal variant
```

- **Wave 1** has zero internal dependencies; all three changes can be dispatched at once to separate subagents.
- **Wave 2** depends only on C1 (schema).
- **Wave 3** depends on C4 and C5.
- **Wave 4** depends on C3 and C6; the two changes inside it are independent.

Total: 8 changes, 4 sequential waves, max-parallel width 3.

---

## Wave 1 — Foundations (parallel)

### C1. Schema extensions — `specify-cli`

**Dependencies:** none. **Parallel with:** C2, C3.

**Scope:**

- Extend `schemas/lint/finding.schema.json`:
  - Widen `status` enum to add `ignored` (keep RFC-28's `open` / `fixed` / `accepted` / `false-positive`).
  - Add optional `disposition` object with `source` (required when present), `directive` (object: `path`, `line`, `rationale`), `since` (optional string).
- Extend `schemas/lint/workspace-model.schema.json`:
  - Add top-level `ignore_directives` array; entries: `path`, `line`, `rule_id`, `rationale?`, `target_line`, `raw`.
- Regenerate any embedded schema snapshots under `crates/schema/src/` if they live there.

**Acceptance:** `cargo make ci` clean; fingerprint algorithm untouched (no field added to fingerprint inputs).

**Files:** `schemas/lint/finding.schema.json`, `schemas/lint/workspace-model.schema.json`, `crates/schema/src/lib.rs` (constants only if needed).

---

### C2. First-party `UNI-*` rules — `specify`

**Dependencies:** none. **Parallel with:** C1, C3.

**Scope:** author two universal rule files pinned by D13. Policy metadata only — they are consumed by the directive-validation pass, not by `kind: regex` hints.

- `adapters/shared/rules/universal/ignore-directive-missing-rationale.md` (id `UNI-022`)
- `adapters/shared/rules/universal/ignore-directive-orphan.md` (id `UNI-023`)

**Style:** follow the existing universal rules under `adapters/shared/rules/universal/` (e.g. `dead-code.md`, `error-message-quality.md`) for frontmatter shape, severity, and prose structure. Cite RFC-33a §"Ignore directives" for rationale.

**Acceptance:** `make check` clean (specdev lint).

---

### C3. Journal `lint-completed` variant — `specify-cli`

**Dependencies:** none. **Parallel with:** C1, C2.

**Scope:** add `lint-completed` to the closed `EventKind` taxonomy in `crates/domain/src/journal.rs` along with a typed payload struct:

```rust
LintCompletedPayload {
    scope: LintScope { target: Option<String>, slice: Option<String>, artifact: Option<String> },
    duration_ms: u64,
    counts: LintCounts { open: u32, ignored: u32, false_positive: u32 },
    baseline_present: bool, // hard-coded false in RFC-33a emitters
    exit_code: i32,
}
```

Wire-id kebab-case (`lint-completed`) joined to a snake_case Rust variant via `#[serde(rename = "lint-completed")]`. No emission yet — wiring is C7.

**Acceptance:** unit test for round-trip serialisation; `cargo make ci` clean. Update `DECISIONS.md` journal-event taxonomy table if one exists.

---

## Wave 2 — Standards-layer DTOs (parallel)

### C4. Finding DTOs — `specify-cli`

**Dependencies:** C1. **Parallel with:** C5.

**Scope:** add `FindingDisposition` / `DirectiveDisposition` DTOs beside `LintFinding` in `crates/specify-lints/src/rules.rs` (and re-export through `crates/specify-lints/src/rules/finding.rs` if that's where the struct lives). Extend the `LintFinding` struct with the optional `disposition: Option<FindingDisposition>` field. Reuse the RFC-28 canonical-JSON helper.

**Status enum:** widen the in-memory `FindingStatus` (or equivalent) to include `Ignored`. Keep serialization byte-stable.

**Acceptance:** unit tests that round-trip an `ignored` finding with directive disposition through canonical JSON.

---

### C5. WorkspaceModel + indexer extractor — `specify-cli`

**Dependencies:** C1. **Parallel with:** C4.

**Scope:**

- Add `IgnoreDirective` DTO and `WorkspaceModel.ignore_directives: Vec<IgnoreDirective>` field in `crates/specify-lints/src/lint/model.rs`.
- Add a new extractor under `crates/specify-lints/src/lint/index/` (e.g. `ignore_directives.rs`) that walks scanned files and emits `IgnoreDirective` facts. Honour the closed comment-style list (D3):
  - `// …` / `/* … */` — C-family
  - `# …` — Shell / Python / YAML / TOML
  - `<!-- … -->` — HTML / Markdown / XML
  - `-- …` — SQL / Lua
- Recognise both em-dash (`—`) and `--` as separator (D3 tolerance).
- Record malformed directives too (rationale missing or short) so the validation pass can emit `UNI-022`.
- `target_line` = next non-blank, non-comment line; inline trailing directives apply to the same line.

**Out of scope:** any post-processing or finding emission — the validation pass owns that (C6).

**Acceptance:** indexer unit tests cover each comment family, em-dash + `--` separator, inline trailing form, blank-line skipping, and malformed-directive capture.

---

## Wave 3 — Scanner pipeline

### C6. Directive-validation pass + status-aware exit — `specify-cli`

**Dependencies:** C4, C5. **No parallel siblings.**

**Scope:** create `crates/specify-lints/src/lint/ignore.rs` implementing the post-hint pass per RFC-33a §"Implementation plan" step 4:

1. Hint evaluation runs as before; every finding starts `status: open`.
2. For each `IgnoreDirective`, locate findings whose `(path, line) == (directive.path, directive.target_line)` and whose `rule_id == directive.rule_id`. Stamp `status: ignored` and populate `disposition.directive`. If rationale begins with `false-positive:`, stamp `status: false-positive` instead.
3. Emit synthetic `UNI-022` finding for any directive whose rationale is missing or shorter than 16 characters.
4. Emit synthetic `UNI-023` finding for any directive whose `rule_id` does not match any finding on its target line.
5. Graceful degradation per RFC-33a §"Graceful degradation": if `UNI-022` / `UNI-023` did not resolve (consumer project without the shared codex tree), do **not** emit the synthetic findings — just skip silently.
6. Update the `specrun lint` exit decision in `src/runtime/commands/lint/run.rs` to status-aware severity: exit 2 only when a finding with `status: open` also has `severity: critical | important`.

**Order in the pipeline:** hint evaluation → default `status: open` → this pass → ordering → envelope/render → exit.

**Acceptance:** module-level tests with synthetic `WorkspaceModel` fixtures; one happy-path golden migrated/added under `tests/`.

---

## Wave 4 — Wiring and acceptance (parallel)

### C7. Journal emission + presentation polish — `specify-cli`

**Dependencies:** C3, C6. **Parallel with:** C8.

**Scope:**

- In `src/runtime/commands/lint/run.rs` (or the equivalent dispatch site), after rendering and before exit, build a `LintCompletedPayload` from the final finding set (counts of `open` / `ignored` / `false_positive`, `baseline_present: false`, captured `duration_ms`, resolved scope, `exit_code` from the new status-aware decision) and append it to the journal via the existing event-emission path.
- Optional small touch in `crates/specify-lints/src/lint/diagnostics/{pretty,compact,github,json}.rs` to render the `status` token next to `severity`. Keep it minimal — RFC-33a explicitly does not require a new flag or formatter shape.

**Acceptance:** integration test invoking `specrun lint run` against a fixture asserts a `lint-completed` line appears in `.specify/journal.jsonl` with the expected payload.

---

### C8. Acceptance golden tests — `specify-cli`

**Dependencies:** C6 (test fixtures need the pass landed). **Parallel with:** C7.

**Scope:** end-to-end goldens per RFC-33a §"Implementation plan" step 7, plus the fingerprint-stability guard from step 1:

1. **No directives present:** baseline fixture → all findings `status: open`; fingerprints unchanged from a pre-RFC-33a snapshot.
2. **Directive matches a finding:** finding flips to `status: ignored`, `disposition.directive` populated.
3. **Directive with `false-positive:` rationale:** finding flips to `status: false-positive`.
4. **Unrationaled / too-short directive:** `UNI-022` synthetic finding emitted.
5. **Orphan directive (no matching finding for its rule-id):** `UNI-023` synthetic finding emitted.
6. **Graceful-degradation fixture:** scan with `UNI-022` / `UNI-023` absent from the resolved codex → no synthetic findings, no scanner error.

Use the existing `REGENERATE_GOLDENS=1` workflow per `docs/standards/testing.md`. Cover all four formatters where they differ materially (`json` and `pretty` at minimum).

**Acceptance:** `cargo make ci` clean on a fresh checkout.

---

## Cross-repo coordination

- C2 lives in `specify`; everything else lives in `specify-cli`. C8's graceful-degradation fixture needs to work both with and without the shared codex tree, so it must not assume `specify` has been checked out alongside.
- The `specify` parent-repo `AGENTS.md` cross-repo update rule (workflow §"Note to the implementing agent") applies: when C4/C5/C6 add or rename symbols touched by the workflow contract, grep both repos and update every hit in the same change.
- Recommended landing order if PRs are sequenced: C1 → (C2 + C3) → (C4 + C5) → C6 → (C7 + C8). If they are batched, Wave 1 can land as a single PR per repo.

## Out of scope (deferred to RFC-33b)

- `.specify/lint/` filesystem state, `baseline` file, `last.json`, `specrun lint baseline …` verbs.
- `status` values `new` / `baselined`, `disposition.baseline` sub-field, `counts.{new, baselined}` in the journal payload, scan-derived `baseline_present`.
- File-, block-, or directory-scoped ignore directives.

## Sequencing summary

| Wave | Change | Repo | Depends on | Parallelisable with |
| --- | --- | --- | --- | --- |
| 1 | C1 Schema extensions | `specify-cli` | — | C2, C3 |
| 1 | C2 `UNI-022` / `UNI-023` rule markdown | `specify` | — | C1, C3 |
| 1 | C3 Journal `lint-completed` variant | `specify-cli` | — | C1, C2 |
| 2 | C4 Finding / Disposition DTOs | `specify-cli` | C1 | C5 |
| 2 | C5 WorkspaceModel + indexer extractor | `specify-cli` | C1 | C4 |
| 3 | C6 Directive-validation pass + exit | `specify-cli` | C4, C5 | — |
| 4 | C7 Journal emission + formatter polish | `specify-cli` | C3, C6 | C8 |
| 4 | C8 Acceptance goldens | `specify-cli` | C6 | C7 |
