# RM-02 Context Implementation Subagent Plan

Purpose: split `rfcs/rm-02-context.md` into small, dependency-aware implementation changes that can each be handed to a separate subagent without over-consuming context.

Implementation repository: `../specify-cli`.

Primary RFC: `rfcs/rm-02-context.md`.

## Planning Constraints

- Keep deterministic behavior in the `specify` binary. Do not add a `/spec:context` skill in this implementation plan.
- Keep context-specific code under `specify-cli/src/commands/context/` unless a later implementation discovers a strong crate-boundary reason.
- Route project-aware commands through the existing `CommandContext` and `run_with_project` flow so legacy-layout checks, version-floor checks, and `--format` behavior stay consistent.
- Preserve the existing CLI exit-code model: success exits `0`, ordinary drift/check failure exits `1`, validation/invocation errors exit `2` only through the established `CliResult`/`Error` mechanisms or an explicit, reviewed extension.
- Use deterministic ordering everywhere: sorted root-marker inputs, sorted registry peers, sorted slice metadata paths, sorted bullets.
- Do not modify the plugin repo's own `AGENTS.md` or `specify-cli/AGENTS.md` as part of implementation.

## Sequencing Overview

```text
Wave 0: Contract decisions
  C00

Wave 1: Minimal command generation
  C01 -> C02 -> C03 -> C04 -> C05 -> C06

Wave 2: Parallel feature enrichment
  C07 and C08 can run in parallel after C06
  C09 can run after C06, and can run in parallel with C07/C08 if its inputs are stable

Wave 3: Staleness contract
  C10 -> C11 -> C12

Wave 4: Final integration and hardening
  C13 -> C14
```

Parallel-safe groups:

- `C07 Detection` and `C08 Hub/workspace enrichment` can run in parallel after the skeleton renderer is stable.
- `C09 JSON schema and docs wiring` can run in parallel with `C07` and `C08` if `context.lock` shape from `C10` is not being changed at the same time; otherwise run it after `C10`.
- Additional tests for already-landed behavior can be split from implementation when a subagent only adds coverage and does not touch production modules.

## Change Packets

### C00: Confirm CLI Contract Decisions

Depends on: none.

Goal: settle the small architectural decisions that affect every later packet.

Scope:

- Confirm `specify context` is project-aware and dispatched via `run_with_project`.
- Confirm `src/commands/context/` can coexist with `src/context.rs` (`CommandContext`) by using explicit imports.
- Decide the implementation mechanism for RM-02's "invocation error exits 2" requirement. Preferred route: keep drift as a non-error `CliResult::GenericFailure` and use existing validation-style errors only for malformed operator input where exit `2` is already appropriate.
- Decide shared workspace-clone detection semantics for `init` integration and `context generate`.

Deliverable:

- A short note in the implementation PR description or a small comment near the new context dispatcher explaining the chosen exit-code and workspace-clone posture.

Subagent prompt:

```text
Review RM-02 and the current specify-cli CLI architecture. Decide the exact command-dispatch, exit-code, and workspace-clone detection posture for `specify context`. Do not implement behavior beyond any tiny comments/helpers needed to lock the decision. Return the chosen decisions and the files touched.
```

### C01: Add Empty CLI Surface

Depends on: C00.

Goal: introduce the command shape without functional generation.

Scope:

- Add `Commands::Context { action: ContextAction }` in `src/cli.rs`.
- Add `ContextAction::{Generate { check: bool, force: bool }, Check}`.
- Add `mod context;` and dispatch in `src/commands.rs`.
- Create `src/commands/context/mod.rs` with stub handlers.
- Stub `generate` should return a clear not-implemented diagnostic or no-op text only long enough for compilation; it must not write files.

Tests:

- Extend existing CLI help or smoke tests only if there is already a nearby pattern.
- Confirm `specify context --help`, `specify context generate --help`, and `specify context check --help` parse.

Subagent prompt:

```text
Implement only the empty RM-02 CLI surface in specify-cli: add `specify context generate [--check] [--force]` and `specify context check`, dispatching through the existing project-aware path. Do not implement rendering, fences, detection, or lock files yet. Add minimal parse/help coverage if consistent with existing tests.
```

### C02: Create Context Test Harness

Depends on: C01.

Goal: create `tests/context.rs` with reusable helpers before behavior grows.

Scope:

- Add `tests/context.rs`.
- Follow integration-test style from `tests/cross_repo.rs`: `assert_cmd::Command::cargo_bin("specify")`, `tempfile`, filesystem assertions, structural assertions.
- Add helpers for:
  - creating a temp project;
  - running `specify init`;
  - reading `AGENTS.md`;
  - asserting fence presence;
  - invoking `specify context generate` and `check`.
- Keep initial tests marked to the stub behavior from C01, or leave helpers plus one parse smoke test if production behavior is not ready.

Deliverable:

- A test harness that later subagents can extend without duplicating setup.

Subagent prompt:

```text
Create the RM-02 integration test harness in `tests/context.rs`. Follow existing specify-cli integration-test conventions and keep assertions limited to behavior already implemented by the current branch. Build reusable helpers for future context tests.
```

### C03: Implement Fence Parser and Writer

Depends on: C01 and C02.

Goal: own safe replacement of generated content before any real renderer writes `AGENTS.md`.

Scope:

- Add `src/commands/context/fences.rs`.
- Implement strict fence detection:
  - opening fence with `<!-- specify:context begin` and key-value lines;
  - closing fence `<!-- specify:context end -->`;
  - preservation of pre-fence and post-fence bytes.
- Implement write policy:
  - absent `AGENTS.md`: create full document;
  - existing unfenced `AGENTS.md`: refuse without `--force`;
  - existing unfenced with `--force`: rewrite whole file;
  - fenced file: replace only generated block and preserve surrounding bytes.
- Do not implement fingerprint lock checks yet. Leave the "fenced content modified" guard for C11.

Tests:

- Unit tests in `fences.rs` for parser edge cases.
- Integration tests for first generation, idempotent fenced regeneration, unfenced refusal, and `--force` overwrite once C04 provides real content.

Subagent prompt:

```text
Implement the RM-02 fenced AGENTS.md parser/writer in `src/commands/context/fences.rs`. Focus only on file-shape policy and byte preservation. Do not add detection or fingerprint locking. Add focused unit tests and wire the module so later renderer work can call it.
```

### C04: Implement Skeleton Renderer

Depends on: C03.

Goal: generate deterministic RM-02 content from existing Specify metadata without root-marker detection or staleness.

Scope:

- Add `src/commands/context/render.rs`.
- Define an internal render input model assembled by `mod.rs`.
- Read:
  - `ProjectConfig`;
  - optional `Registry`;
  - capability/pipeline data for non-hub projects;
  - active slice count by reading slice names only.
- Render regular project sections:
  - Runtime;
  - Tests;
  - Linting;
  - Navigation;
  - Conventions;
  - Boundaries;
  - Dependencies.
- Runtime/Tests/Linting render `not detected` placeholders until C07.
- Render hub variant without Runtime/Tests/Linting.
- Sort every bullet deterministically.

Tests:

- Regular single-repo generation has seven headings.
- Multi-repo registry peers appear under Dependencies.
- Hub generation omits Runtime/Tests/Linting.
- Re-running generation is byte-identical.

Subagent prompt:

```text
Implement RM-02 skeleton rendering for `specify context generate`. Use project config, optional registry, capability/pipeline data, and active slice count only. Runtime/Tests/Linting should say `not detected`. Support hub shape. Do not implement root-marker detection or lock files yet.
```

### C05: Wire Generate Behavior

Depends on: C04.

Goal: make `specify context generate` useful and stable for Phase 1.

Scope:

- Connect `ContextAction::Generate` to render + fenced write.
- Emit concise text output.
- For `--format json`, use the existing JSON envelope and kebab-case keys.
- Make `generate --check` compare the would-be `AGENTS.md` bytes without writing and exit `1` when a write would occur.
- Keep `context check` as a stub until C10/C11.

Tests:

- `generate` writes `AGENTS.md`.
- `generate --check` succeeds when clean and fails when generation would update the file.
- JSON output is wrapped with `schema-version`.

Subagent prompt:

```text
Wire the RM-02 skeleton renderer into `specify context generate`, including `--check`, `--force`, text output, and JSON output through the existing response envelope. Leave `context check` as a clear stub until fingerprint work lands.
```

### C06: Add Init Integration

Depends on: C05.

Goal: freshly initialized projects get generated context automatically.

Scope:

- In `src/commands/init.rs`, after successful `specify::init(opts)?`, call the same context generation path.
- Generate only when root `AGENTS.md` is absent.
- If `AGENTS.md` already exists, skip and print one concise note in text mode; include a structured skipped flag or warning in JSON only if it fits existing output style.
- Hub init should use the hub renderer.
- Init inside `.specify/workspace/<peer>/` should not generate a nested `AGENTS.md`.
- Do not run context generation from migrators.

Tests:

- `specify init <capability>` creates `AGENTS.md`.
- `specify init --hub` creates hub-shaped `AGENTS.md`.
- Pre-existing `AGENTS.md` remains byte-for-byte unchanged.
- Workspace clone init skips generation.

Subagent prompt:

```text
Integrate RM-02 context generation into `specify init`: after successful init, generate AGENTS.md only when absent, skip when present, support hubs, and skip workspace clones. Reuse the context generate path rather than duplicating rendering logic.
```

### C07: Implement Root-Marker Detection

Depends on: C06.

Can run in parallel with: C08.

Goal: replace Runtime/Tests/Linting placeholders with factual detected guidance.

Scope:

- Add `src/commands/context/detect.rs`.
- Scan only the project root and the fixed allowlist from the RFC.
- Detect:
  - Rust: `Cargo.toml`, optional `rust-toolchain.toml`, `clippy.toml`;
  - Node: `package.json`, `engines.node`, test/lint scripts where present;
  - Python: `pyproject.toml` and `requirements.txt`;
  - Go: `go.mod`;
  - Deno: `deno.json` / `deno.jsonc`;
  - Make: `Makefile` `test` and `checks` targets;
  - GitHub Actions: first workflow name;
  - linter markers from RFC allowlist.
- Detector returns structured data and warnings. Renderer maps data to bullets.
- On corrupt marker files, render `not detected` for that marker and emit a warning; do not guess.

Tests:

- Cargo project detects Rust, `cargo test`, and `cargo clippy`.
- npm project detects Node, `npm test`, and configured lint.
- Mixed-language project orders Runtime bullets deterministically.
- Corrupt TOML/JSON/YAML marker emits warning and does not panic.

Subagent prompt:

```text
Implement RM-02 root-marker detection under `src/commands/context/detect.rs` and connect it to Runtime, Tests, and Linting rendering. Keep scans shallow, deterministic, and allowlist-only. Corrupt marker files should warn and render `not detected`, not guess.
```

### C08: Enrich Hub, Dependencies, and Workspace Navigation

Depends on: C06.

Can run in parallel with: C07.

Goal: complete non-detection rendering details from the RFC.

Scope:

- Ensure hub projects permanently omit Runtime/Tests/Linting.
- Enrich Dependencies with registry peer descriptions where present.
- Add `.specify/workspace/<peer>/` paths to Navigation when materialized.
- Ensure Navigation includes repo-root platform artifacts and `.specify/` paths using repo-relative paths only.
- Keep capability-specific prose out of the binary.

Tests:

- Hub with two peers lists both dependencies with descriptions.
- Synced workspace clones appear under Navigation.
- Single-repo projects render `single-repo project; no registered peers`.

Subagent prompt:

```text
Finish RM-02 rendering details for hubs, dependencies, and workspace navigation. Do not touch root-marker detection or fingerprint locking. Keep all paths repo-relative and all bullet ordering deterministic.
```

### C09: Add Context Lock Schema Shell

Depends on: C05.

Can run in parallel with: C07 and C08 if C10 has not started; otherwise run after C10.

Goal: prepare the distributed schema artifact without coupling it to implementation internals too early.

Scope:

- Add `schemas/context-lock.schema.json`.
- Update `schemas/README.md`.
- Match the lock shape from RM-02:
  - `version`;
  - `fingerprint`;
  - `cli_version`;
  - `inputs[]`;
  - `fences.body_sha256`.
- Keep lock-file property names aligned with the serialized YAML shape. The RM-02 RFC uses snake_case for `cli_version` and `body_sha256`; JSON command output remains kebab-case through the existing CLI envelope conventions.

Tests:

- If the repo has schema validation tests, include the new schema in that path.
- Otherwise keep this packet docs/schema-only.

Subagent prompt:

```text
Add the RM-02 context lock JSON Schema artifact and schema README entry. Align property names with the planned serialized `.specify/context.lock` shape. Do not implement fingerprinting in this packet.
```

### C10: Implement Fingerprint Input Collection

Depends on: C07 and C08.

Goal: collect and hash the exact renderer inputs deterministically.

Scope:

- Add `src/commands/context/fingerprint.rs`.
- Track every file actually read by rendering:
  - `.specify/project.yaml`;
  - `registry.yaml` when present;
  - `plan.yaml` presence/content according to the final renderer contract;
  - detected root-marker files;
  - resolved `capability.yaml` for non-hub projects;
  - `.specify/slices/*/.metadata.yaml` files.
- Compute per-input SHA-256.
- Compute the aggregate fingerprint using the canonical recipe from RM-02.
- Compute `fences.body_sha256`.
- Keep fingerprint code deterministic and unit-tested.

Tests:

- Stable aggregate hash for sorted inputs.
- Adding/removing/reordering input collection does not change output except by documented input changes.
- Body hash changes when fenced body changes.

Subagent prompt:

```text
Implement RM-02 fingerprint input collection and hashing in `src/commands/context/fingerprint.rs`. Track only files the renderer actually reads, sort by repo-relative path, compute per-input sha256, aggregate fingerprint, and fenced body sha. Add focused unit tests.
```

### C11: Implement Lock Read/Write and Check Semantics

Depends on: C10.

Goal: make `.specify/context.lock`, `context check`, and fenced drift detection authoritative.

Scope:

- Serialize `.specify/context.lock` as YAML using the repo's YAML conventions.
- `generate` writes or refreshes the lock after successful AGENTS write.
- `generate --check` validates whether a write or lock refresh would be needed without writing.
- `context check` exits:
  - `0` when lock exists and current state matches;
  - `1` for missing `AGENTS.md`, missing lock, input drift, or fenced body drift;
  - `2` only for invocation/malformed-lock conditions via the agreed C00 mechanism.
- JSON output should follow the RM-02 shape and existing envelope:
  - `status`;
  - `fingerprint.expected`;
  - `fingerprint.actual`;
  - `inputs-changed`;
  - `inputs-added`;
  - `inputs-removed`;
  - `fences-modified`.
- `generate` should refuse to overwrite modified fenced content unless `--force` is set.

Tests:

- Generate, mutate `registry.yaml`, check reports drift on `registry.yaml`.
- Generate, edit between fences, check reports `fences-modified: true`.
- Generate twice, then check exits `0`.
- Missing `AGENTS.md` and missing lock get distinct statuses.
- Newer lock version is rejected with `context-lock-version-too-new` or the finalized diagnostic name.

Subagent prompt:

```text
Implement RM-02 `.specify/context.lock`, `specify context check`, `generate --check` lock validation, and fenced-content drift refusal. Preserve existing JSON envelope conventions and the agreed exit-code behavior. Add integration coverage for clean, input drift, fenced drift, missing file, missing lock, and too-new lock version.
```

### C12: Add Atomic Writes and Error Variants

Depends on: C11.

Goal: harden persistence and diagnostics once the full write set is known.

Scope:

- Ensure `AGENTS.md` and `.specify/context.lock` writes that consumers may read are atomic where appropriate, using the repo's `NamedTempFile::new_in(parent).persist(target)` convention.
- Add stable error variants to `crates/error/src/lib.rs` only for diagnostics that tests or skills need to grep:
  - `context-existing-unfenced-agents-md`;
  - `context-fenced-content-modified`;
  - `context-lock-missing`;
  - `context-not-generated`;
  - `context-lock-version-too-new`;
  - any finalized malformed-lock diagnostic.
- Update `variant_str()` and unit tests for new variants.

Tests:

- Error variant string tests.
- Integration tests assert stable diagnostic names, not long prose.

Subagent prompt:

```text
Harden RM-02 persistence and diagnostics. Make AGENTS.md/context.lock writes atomic where appropriate, add stable context error variants needed by tests and consumers, update `variant_str()`, and revise tests to assert diagnostic identifiers.
```

### C13: Full Acceptance Coverage

Depends on: C07, C08, C11, C12.

Goal: make `tests/context.rs` cover the full RM-02 V1 contract.

Scope:

- Complete the acceptance matrix from `rfcs/rm-02-context.md`:
  - regular init/generate;
  - hub init/generate;
  - pre-existing `AGENTS.md` skip on init;
  - idempotent generation;
  - unfenced refusal and `--force`;
  - Cargo detection;
  - registry drift;
  - fenced-content drift;
  - clean check;
  - hub dependencies with descriptions.
- Prefer structural assertions over full prose snapshots.
- Keep temp paths out of expected output.

Subagent prompt:

```text
Complete RM-02 acceptance coverage in `tests/context.rs` against the current implementation. Prefer structural assertions over byte-for-byte prose checks. Cover regular projects, hubs, init behavior, fences, detection, lock drift, clean checks, and dependency rendering.
```

### C14: Final Documentation and Cleanup

Depends on: C13.

Goal: finish operator-facing docs and remove phase scaffolding.

Scope:

- Ensure `specify context --help` text is operator-facing and not RFC-internal.
- Add or update CLI reference docs in the plugin repo if this repository maintains generated/manual CLI docs for new verbs.
- Remove temporary stub messages such as `context-not-implemented`.
- Confirm no implementation packet modified hand-authored root `AGENTS.md` files.
- Run formatting and targeted tests.

Verification:

- `cargo make fmt`
- `cargo make test` or, if too expensive for the final subagent, `cargo nextest run --test context`
- Any schema/doc checks required by the touched repository.

Subagent prompt:

```text
Finalize RM-02 after implementation: clean up temporary messages, verify help text, update CLI docs if this repo maintains them, ensure no hand-authored AGENTS.md files were modified, run formatting, and run targeted context tests.
```

## Recommended Parallel Execution

### Wave 0

Run `C00` alone. It decides behavior later packets must not contradict.

### Wave 1

Run `C01`, then `C02`, then `C03`/`C04`/`C05`/`C06` sequentially. These packets touch the same command surface and should land in order to avoid merge churn.

### Wave 2

After `C06`, run these in separate subagents:

- Subagent A: `C07 Detection`.
- Subagent B: `C08 Hub/workspace enrichment`.
- Subagent C: `C09 Context lock schema shell` if fingerprint serialization names are already agreed.

Merge order for Wave 2: `C08`, then `C07`, then `C09`, unless `C09` depends on property names finalized during `C10`.

### Wave 3

Run `C10`, `C11`, and `C12` sequentially. These packets share the fingerprint and lock contract, so parallel work here is likely to create conflicting assumptions.

### Wave 4

Run `C13` and `C14` sequentially. `C13` should expose any behavior gaps before final cleanup.

## Suggested First Implementation Prompt

```text
Implement C00 and C01 from `rfcs/rm-02-context-subagent-plan.md`.

Goal: add only the empty `specify context` CLI surface for RM-02 in `specify-cli`.

Scope:
- Read `rfcs/rm-02-context.md` and `rfcs/rm-02-context-subagent-plan.md`.
- Add `Commands::Context { action: ContextAction }`.
- Add `ContextAction::Generate { check: bool, force: bool }` and `ContextAction::Check`.
- Dispatch through the existing project-aware path in `src/commands.rs`.
- Add a new `src/commands/context/mod.rs` with stub handlers only.
- Do not implement rendering, fences, root-marker detection, init integration, or lock files.
- Add minimal parse/help coverage if it fits existing test patterns.

Return the decisions made for C00 and the files changed for C01.
```

