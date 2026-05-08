# RM-02 `specify context` Implementation Plan

> Purpose: handoff notes for implementing the `specify context generate` / `specify context check` verbs called for by RM-02 in `rfcs/roadmap.md`.

## Context

RM-02 introduces the first deterministic generator for repository-level agent context. The roadmap entry is short:

> **Goal:** Generate concise, deterministic, refreshable repository context.
> **Inputs:** Specify project metadata, capability references, repo inspection, and registry data.
> **Output:** short `AGENTS.md` guidance covering runtime, tests, linting, navigation, conventions, boundaries, and dependencies. The proposed `specify context check` warns when repo changes imply a refresh.
> **Why now:** High direct user value, and it unblocks stale-context checks in `specify review`.

Two roadmap principles bound the design:

- **Keep the CLI authoritative.** `specify context` is a deterministic CLI verb, not an agent skill. A `/spec:context` skill may wrap it later for prose-fill holes, but the canonical generator is the binary.
- **Do not make `AGENTS.md` long-form documentation.** The output is short, factual, and refreshable. Operators write narrative docs elsewhere.

The roadmap also calls out a downstream consumer: `specify review` (RM-11) needs a "stale `AGENTS.md`" finding. RM-02's staleness primitive is the contract that satisfies that consumer.

## Scope

### In scope

- `specify context generate` — write or refresh `AGENTS.md` at the project root.
- `specify context check` — exit non-zero when the recorded inputs have drifted.
- `--check` flag on `generate` for CI dry-run usage.
- Fenced overwrite policy that preserves operator-authored prose appended below the generated block.
- A small fingerprint sidecar at `.specify/context.lock` so review tooling has a structured drift signal.

### Out of scope

- Codex rule embedding (RM-03 owns the rule format and storage).
- `specify review` integration (RM-11) — RM-02 only exposes the staleness primitive.
- Any cross-repo `AGENTS.md` orchestration. Each peer in a multi-repo registry runs its own `specify context generate`.
- Long-form documentation, narrative onboarding guides, or per-feature READMEs. Roadmap §Non-Goals is explicit.
- LLM-driven prose generation. Phase 5 may add a `/spec:context` skill that fills prose holes; the V1 binary is purely template-driven.

## Target surface

```bash
specify context generate           # write AGENTS.md
specify context generate --check   # dry-run; exit non-zero if writes would change
specify context generate --force   # rewrite even when fences are missing
specify context check              # alias for the staleness path
```

Wired in `src/cli.rs` as:

```text
Commands::Context { action: ContextAction }

ContextAction::
  Generate { check: bool, force: bool }
  Check
```

`--format text|json` is inherited from the global flag (`Cli::format`), matching `specify status` and `specify slice status`.

## Init integration

`specify init` (and the `/spec:init` skill that wraps it) generates `AGENTS.md` at the end of a successful init run, but only when the file is absent. The posture is:

1. After `init` finishes scaffolding `.specify/` and writing `project.yaml`, it runs the same code path as `specify context generate` against the freshly-initialised project.
2. When `AGENTS.md` already exists at the project root (fenced or not), `init` skips generation and prints a one-line note (`AGENTS.md already present; skipping context generate`). It never refuses, never overwrites, never warns.
3. Hub projects (`--hub`) get the hub variant of `AGENTS.md`, matching the §"Hub variant" rules.
4. `init` inside a workspace clone (`.specify/workspace/<peer>/`) does not generate. Workspace clones inherit context from their owning peer's `init` and should not maintain a parallel AGENTS.md.
5. Init-time generation honours the same fingerprint contract as a standalone `generate`: it writes `.specify/context.lock` so a follow-up `specify context check` is green out of the box.

Why this posture:

- Every freshly-initialised Specify project has an AGENTS.md from minute zero. RM-11's stale-context check can assume the file exists on every project.
- The "skip when present" rule means re-running `init` (e.g. with a different `--name`) and copying an AGENTS.md into a fresh dir before init both stay safe.
- `migrate {v2-layout, slice-layout, change-noun}` deliberately do **not** auto-generate AGENTS.md. Migrators stay focused on layout; operators on existing projects run `specify context generate` once after migrating.

The init wiring lives in `src/commands/init.rs`, calling into `commands::context::run_generate` with a `from-init` flag that downgrades the existing-AGENTS-md guard from "refuse" to "skip with note".

## Implementation surface

### New / changed files

| File | Change |
|---|---|
| `src/cli.rs` | Add `Commands::Context`, `ContextAction` |
| `src/commands.rs` | Dispatch the new arm |
| `src/commands/init.rs` | Tail-call `context::run_generate` when `AGENTS.md` is absent |
| `src/commands/context.rs` | New module — entry points `run_generate`, `run_check` |
| `src/commands/context/render.rs` | Pure rendering of the seven sections |
| `src/commands/context/detect.rs` | Root-marker toolchain detection |
| `src/commands/context/fingerprint.rs` | Hashing + `.specify/context.lock` IO |
| `src/commands/context/fences.rs` | Fence parser / writer |
| `tests/context.rs` | Integration tests |
| `schemas/context-lock.schema.json` | JSON Schema for the lock file |

### Crate boundary

V1 keeps the logic inside the binary, mirroring `src/commands/status.rs`. The hard rule is that nothing outside `src/commands/context/` should reach into context-specific state. If the file count grows past ~6 we lift to `crates/context/`. The lift is mechanical — the binary already hosts `ProjectConfig` and the dispatcher and we don't want to invent a crate boundary before the shape settles.

## Inputs

V1 reads only files Specify already knows about plus a flat scan of the project root. No deep tree walking, no external network, no shelling out to language toolchains.

| Input | Source | Crate |
|---|---|---|
| Project name, capability id, domain, hub flag, declared tools | `.specify/project.yaml` | `src/config.rs::ProjectConfig` |
| Resolved capability + pipeline | `specify_capability::resolve` against the cached / bundled tree | `specify_capability::Capability` |
| Registry peers | `registry.yaml` (optional) | `specify_registry::Registry` |
| Active slice inventory (names + lifecycle status) | `.specify/slices/*/.metadata.yaml` | `commands::slice::collect_status` |
| Plan presence (only the file's existence — no per-entry detail) | `plan.yaml` | `specify_change::Plan::load` |
| Root markers | `read_dir(project_dir)` filtered to a fixed allowlist | new |

Active slice inventory is read only to surface the slice count under **Navigation**. We deliberately do not enumerate per-slice status — that already lives in `specify status` and would explode the fingerprint surface.

### Root-marker allowlist (V1)

| Marker | Implies |
|---|---|
| `Cargo.toml` | Rust toolchain (read `rust-toolchain.toml` channel if present) |
| `package.json` | Node.js (read `engines.node` if present) |
| `pyproject.toml` | Python; pick `tool.poetry` / `[project]` for command hints |
| `requirements.txt` | Python (fallback) |
| `go.mod` | Go (read `go` directive) |
| `deno.json` / `deno.jsonc` | Deno |
| `Makefile` | shell out hint: `make test`, `make checks` if those targets exist (parsed as plain text) |
| `.github/workflows/*.yaml` | "linted by GitHub Actions"; cite the first workflow name |
| `clippy.toml`, `.eslintrc*`, `ruff.toml`, `deno.json:lint` | linter hint |

The detector returns a structured `Detection` value; the renderer maps it to bullet text. Anything not detected renders as the literal string `"not detected"`. The detector never guesses.

## Output

### File shape

```markdown
# <project-name> — Agent Instructions

<!-- specify:context begin
fingerprint: sha256:abcd…
generated-by: specify <semver>
-->

## Runtime
- …

## Tests
- …

## Linting
- …

## Navigation
- …

## Conventions
- …

## Boundaries
- …

## Dependencies
- …

<!-- specify:context end -->

<!-- Operator-authored prose may follow this line; `specify context generate`
     never touches anything below the closing fence. -->
```

### Section contract

| Section | Source | V1 content |
|---|---|---|
| Runtime | `Detection` | Detected toolchain + version pin if present |
| Tests | `Detection` | Single command line (e.g. `cargo test`, `make test`) or "no test command detected" |
| Linting | `Detection` | Single command line (e.g. `make checks`, `cargo clippy`) or "no lint command detected" |
| Navigation | `ProjectConfig` paths + slice count | Bullets for `.specify/slices/`, `.specify/archive/`, `plan.yaml`, `change.md`, `registry.yaml`, peer paths, plus active-slice count |
| Conventions | `project.yaml.rules`, capability briefs | Links to `.cursor/rules/*` (when present), each `rules:` override file, capability brief titles from `specify capability pipeline` |
| Boundaries | Fixed list + capability + tools | Never-hand-edit list (`.metadata.yaml`, `.specify/archive/`, capability outputs), declared WASI tools, capability ownership note |
| Dependencies | `registry.yaml` | One bullet per peer (`name @ schema → url`) or `single-repo project; no registered peers` |

### Hub variant

Hub projects (`project.yaml.hub: true`) drop **Runtime**, **Tests**, and **Linting** — they hold no source code. They keep all four other sections; **Dependencies** becomes the primary content because peers are the hub's reason for existing.

### Determinism contract

- Output bytes are a pure function of `(inputs, CLI version)`.
- `generate` is a no-op on a clean tree: re-running produces byte-identical bytes between the fences.
- No timestamps, no `$USER`, no host info, no machine paths. Repo-relative paths only.
- Section order is fixed.
- Bullet order within a section is sorted (lexicographic on the deterministic key, which is documented per section in `render.rs`).

## Fingerprint and staleness

### Lock file

`.specify/context.lock` is YAML with this shape (v1):

```yaml
version: 1
fingerprint: sha256:…
cli_version: 0.4.2
inputs:
  - path: .specify/project.yaml
    sha256: …
  - path: registry.yaml
    sha256: …
  - path: Cargo.toml
    sha256: …
  - path: .github/workflows/ci.yaml
    sha256: …
fences:
  body_sha256: sha256:…   # the rendered content between the fences
```

- `inputs` lists every file the renderer actually read, sorted by path.
- Files that exist but are unread (e.g. workflow yamls beyond the first one we summarise) are **not** listed. The fingerprint must reflect what the renderer consumed, nothing more.
- `fingerprint` is a single sha256 over the canonical encoding `{cli_version}\n{path1}\t{sha1}\n{path2}\t{sha2}\n…`. The exact recipe is in `fingerprint.rs` and unit-tested.
- `fences.body_sha256` lets `check` detect operator edits inside the fences without re-rendering. We need both: a renderer-input drift and a hand-edit drift.

### `specify context check` semantics

Exit codes:

| Code | Condition | Message |
|---|---|---|
| 0 | Lock present and current | `context up to date` |
| 1 | Inputs drifted (`fingerprint` mismatch) | per-input drift list |
| 1 | Fences drifted (`body_sha256` mismatch) | `context-fenced-content-modified` |
| 1 | `AGENTS.md` missing | `context-not-generated` |
| 1 | Lock missing | `context-lock-missing` |
| 2 | Invocation error (read failure, malformed lock) | error name |

JSON shape (`--format json`):

```json
{
  "status": "drift",
  "fingerprint": { "expected": "sha256:…", "actual": "sha256:…" },
  "inputs_changed": ["registry.yaml"],
  "inputs_added": [],
  "inputs_removed": [],
  "fences_modified": false
}
```

`generate --check` has the same semantics but does not write anything. It is the CI-friendly form: green when re-running `generate` would be a no-op, red otherwise.

## Fence policy

### Why fences

Operators want to keep project-specific guidance in `AGENTS.md`. A fence-managed block lets the generator own its section while leaving operator prose untouched.

### Rules

1. The first `generate` on a project without `AGENTS.md` writes a fenced document with the seven sections and an empty trailing area.
2. The first `generate` on a project with an existing un-fenced `AGENTS.md` refuses unless `--force` is set. The error is `context-existing-unfenced-agents-md`.
3. With `--force` and an existing un-fenced `AGENTS.md`, the entire file is rewritten and the prior contents are lost. This is intentional — operators who want to preserve content must add fences manually before re-running.
4. With fences present, `generate` rewrites only the contents between the fences. Anything before the opening fence (header) and after the closing fence (operator prose) is preserved byte-for-byte.
5. When `body_sha256` in the lock disagrees with the file's current inter-fence bytes, `generate` refuses with `context-fenced-content-modified` so the operator can reconcile. `--force` overrides.
6. The header above the fences is itself templated (`# <name> — Agent Instructions`) and is written only on first generation. Subsequent runs do not touch it.

### Fence syntax

```text
<!-- specify:context begin
<key: value pairs, one per line>
-->
…
<!-- specify:context end -->
```

Implemented as a strict regex match in `fences.rs`. The opening fence carries the embedded `fingerprint:` for human-readable parity with the lock file; the lock file is still authoritative for `check`.

## Phasing — ship Phase 1 first

Each phase ends with a green `tests/context.rs` that exercises the new behaviour. Earlier-phase tests remain green.

### Phase 1 — Skeleton

- CLI wiring (`Commands::Context`, dispatcher, no-op `Check`).
- `Generate` writes a fenced `AGENTS.md` with the seven section headings populated from `ProjectConfig`, `Capability`, and `Registry` only.
- `init` tail-calls `Generate` when `AGENTS.md` is absent (see §"Init integration").
- No detection. Runtime/Tests/Linting bullets render `not detected`.
- No staleness. `Check` exits 0 with `context-not-implemented` and the lock file is not yet written.
- Tests:
  - regular project (single-repo): file written, structure correct.
  - regular project (multi-repo): peers appear under Dependencies.
  - hub project: Runtime/Tests/Linting absent.
  - `specify init` on a fresh project leaves `AGENTS.md` populated.
  - `specify init` on a project with a pre-existing `AGENTS.md` leaves the file untouched.
  - re-running on a fenced file is a no-op (byte-identical output).
  - re-running on an un-fenced file errors without `--force`.

### Phase 2 — Detection

- Implement `Detection` over the root-marker allowlist.
- Runtime/Tests/Linting bullets become factual.
- Tests:
  - Cargo project picks up `cargo test` + clippy.
  - npm project picks up `npm test` + eslint when configured.
  - Mixed-language projects render multiple Runtime bullets in sorted order.
  - Detection failure for a marker (corrupt YAML/TOML) renders `not detected` and emits a warning to stderr.

### Phase 3 — Staleness

- Add `fingerprint.rs` + `.specify/context.lock`.
- `Generate` writes the lock; `Generate --check` and `Check` consume it.
- Tests:
  - Generate, mutate `registry.yaml`, `check` reports drift on the right path.
  - Generate, edit between fences, `check` reports `fences_modified: true`.
  - Generate twice without changes — the second `check` is green.
  - Lock-version forwards-compatibility: a lock with a newer `version:` field is rejected with `context-lock-version-too-new`.

### Phase 4 — Hub variant + workspace peers

- Hub-shape rendering.
- Dependencies enriched with `description:` from `RegistryProject`.
- Workspace peer paths under Navigation when `.specify/workspace/<peer>/` exists.
- Tests:
  - Hub fixture with two peers: Dependencies lists both with descriptions.
  - Synced workspace clones appear under Navigation.

### Phase 5 — Skill + review hook

Out-of-scope for the binary work but listed for sequencing.

- Optional `/spec:context` skill in `plugins/spec/` that runs `specify context generate` and offers explicit prose-fill holes between marker comments inside the fenced block (e.g. `<!-- specify:context fill conventions -->…<!-- end fill -->`).
- The fill regions are content-preserving across regenerate just like the post-fence operator prose, but they live inside the fenced area and *do* participate in the fingerprint, so changes are detected.
- `specify review` (RM-11) wires `specify context check --format json` into its findings stream once RM-04's finding schema lands.

## Acceptance test

`specify-cli/tests/context.rs` (new). Mirrors the structure of `tests/cross_repo.rs`: temp project, real `specify` binary, structural assertions only.

```text
context_regular
  init regular project
  AGENTS.md exists (written by init); contains all seven section headings;
  contains opening + closing fences;
  no .specify/context.lock yet (Phase 3 adds it).

context_hub
  init --hub
  AGENTS.md does not contain Runtime / Tests / Linting headings.

context_init
  pre-write a hand-authored AGENTS.md;
  specify init <capability>;
  AGENTS.md is unchanged byte-for-byte;
  init prints `AGENTS.md already present; skipping context generate`.

context_idempotent
  init; record bytes; specify context generate; bytes unchanged.

context_unfenced_refuses
  hand-write AGENTS.md without fences;
  generate -> non-zero with `context-existing-unfenced-agents-md`;
  generate --force -> overwrites; subsequent generate is idempotent.

context_detect_cargo
  generate in a project containing Cargo.toml + clippy.toml;
  Runtime line names Rust; Tests line is `cargo test`; Linting line is `cargo clippy`.

context_drift_registry
  generate;
  rewrite registry.yaml;
  context check -> exit 1, JSON inputs_changed = ["registry.yaml"].

context_drift_fences
  generate;
  edit content between fences;
  context check -> exit 1, JSON fences_modified = true.

context_clean
  generate; check; exit 0.

context_hub_deps
  init --hub; registry add two peers with descriptions;
  generate; Dependencies section lists both with their descriptions.
```

## Open decisions

- **Lock file vs header comment for the canonical fingerprint.** Recommend the lock file (matches `plan.lock` ergonomics; lets `check` run without parsing markdown). The header comment is informational only.
- **`AGENTS.md` location for hubs vs regular projects.** Both at the repo root. The hub variant differs in content, not location.
- **Whether language detection is in the binary or pluggable per capability.** V1 puts it in the binary. Capability-pluggable detection is deferred — Omnia/Vectis can override Runtime/Tests/Linting via a capability hook in a follow-up RFC.
- **Migration of existing `AGENTS.md`.** This repo's `AGENTS.md` (and `specify-cli/AGENTS.md`) are intentionally hand-authored long-form documents. We do not auto-migrate them. `init` skips generation when the file is present (see §"Init integration"); a manual `specify context generate` on those projects refuses without `--force`. Operators decide whether to fence-manage their AGENTS.md or keep authoring it by hand. `specify migrate {v2-layout, slice-layout, change-noun}` deliberately do not auto-generate.
- **Sort key for multi-language Runtime detection.** Recommend kebab-case language id ascending: `go`, `node`, `python`, `rust`. Documented in `render.rs`.

## Risks

### Detection fragility

Root-marker inference is approximate. A project may carry a `Makefile` without a `test` target, or a `Cargo.toml` for a non-default workspace member. Counter:

- Detection never guesses. When a marker is present but the substructure is unclear, render `not detected` rather than something wrong.
- All detected commands are presented as bullets prefixed by `detected:` so operators reading `AGENTS.md` know the line is generated and may need overriding via post-fence prose.

### Fence-content drift

Operators may edit between the fences for legitimate reasons. The default refuses (with diagnostic) so we never silently lose work; `--force` rewrites. A future Phase 5 fill-region mechanism gives a non-destructive escape hatch.

### Capability coupling

Easy to bleed capability-specific prose into the binary. Discipline:

- The binary may read `Capability` and surface its `description` and brief titles, but never embeds capability-specific guidance.
- Anything that varies between Omnia and Vectis goes through a capability hook (Phase 5 work, not V1).

### Fingerprint surface explosion

The temptation is to fingerprint everything. We cap V1 at:

- `.specify/project.yaml`
- `registry.yaml`
- `plan.yaml` (presence only — its sha lands in the fingerprint, but the renderer does not unpack the entries)
- The detected root-marker files
- The capability manifest (`capability.yaml`) at its resolved path
- The `.specify/slices/*/.metadata.yaml` files (so adding a slice triggers refresh)

Anything else is out. If a future need surfaces, it goes through an RFC amendment, not a quiet fingerprint addition.

## Relationship to other roadmap items

- **RM-03 (codex rules)** will give the Conventions section richer content. RM-02 must not assume the codex format — Conventions stays template-driven on `project.yaml.rules` for V1.
- **RM-04 (review finding schema)** standardises how `context check` drift surfaces in `specify review`. RM-02 ships the JSON shape documented above; RM-11 maps it to findings without RM-02 changing.
- **RM-11 (`specify review`)** is the headline consumer of staleness. RM-02 ships the staleness primitive; review wires it in later.
- **RM-22 (capability ecosystem)** may add capability-pluggable detection. RM-02 deliberately does not pre-bake that hook.

## Suggested next prompt for an implementer

```text
Implement Phase 1 of RM-02 from rfcs/rm-02-context.md.

Scope: add `specify context generate` and a stub `specify context check`
that exits 0 with `context-not-implemented`. The generator writes a
fenced AGENTS.md at the project root from .specify/project.yaml,
registry.yaml, and the resolved Capability. No detection (Runtime /
Tests / Linting render `not detected`). No fingerprint, no lock file.

Init integration: `specify init` tail-calls `context::run_generate`
when AGENTS.md is absent at the project root, and skips silently
when present. Refuses to generate inside a workspace clone. See
§"Init integration" for the full posture.

Wiring: mirror `src/commands/status.rs` for dispatch shape. Place new
code under `src/commands/context/` as `mod.rs`, `render.rs`,
`fences.rs`. Update `src/cli.rs`, `src/commands.rs`, and
`src/commands/init.rs`.

Tests: add `tests/context.rs` covering the Phase 1 scenarios listed in
the rm-02 plan (`context_regular`, `context_hub`, `context_init`,
`context_idempotent`, `context_unfenced_refuses`).

Do not modify the repo's own AGENTS.md or specify-cli/AGENTS.md as
part of the change. Do not add a /spec:context skill yet.
```
