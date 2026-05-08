# RT Plugin

Fixture capture and regression testing for migrations. The RT plugin supports the migration workflow by providing tools to capture runtime fixtures from a legacy service and write regression tests from those fixtures. Repository cloning is no longer a dedicated skill — both callers inline a guarded `git clone` snippet directly; see the *Cloning a source tree* subsection in [`plugins/spec/skills/analyze/SKILL.md`](../../../plugins/spec/skills/analyze/SKILL.md) (or [`plugins/rt/skills/wiretapper/SKILL.md`](../../../plugins/rt/skills/wiretapper/SKILL.md) for legacy-repo bootstrap).

## Skills

### /rt:wiretapper

Capture request/response and side-effect data from a legacy TypeScript service.

**Synopsis:**

```text
/rt:wiretapper <legacy-dir> [app-name <name>]
```

**Inputs:**
- `legacy-dir` -- Path to the legacy TypeScript project.
- `--app-name` -- Name for the captured fixture file.

**Outputs:**
- `src/wiretap/` directory with core capture logic and per-pattern adapters.
- Modified entry point that routes through the wiretap layer.
- At runtime: `<app>.wiretap.json` containing captured request/response pairs and side effects.

**Behavior:**
1. Detects patterns in the legacy code (HTTP handlers, message consumers, WebSocket, etc. -- patterns A through H).
2. Generates wiretap adapters for each detected pattern.
3. Wires the wiretap into the application entry point.
4. Verifies the modified project compiles.

The captured fixtures serve as input to the replay-writer skill.

### /rt:replay-writer

Add regression tests from captured JSON fixtures.

**Synopsis:**

```text
/rt:replay-writer <crate-name> [project-dir <path>]
```

**Inputs:**
- `crate-name` -- The Omnia crate to test.
- `--project-dir` -- Project directory (defaults to current directory).
- Reads fixtures from `tests/data/replay/`.

**Outputs:**
- New or updated test files that replay captured fixtures against the new implementation.
- Passing `cargo test` suite.

**Behavior:**
1. Inspects the crate and its existing tests.
2. Reads fixture files from `tests/data/replay/`.
3. Generates test cases that replay each captured request and assert the response matches.
4. Runs `cargo test` and iterates on failures.

## Migration workflow

The two RT skills form a pipeline (preceded by an inlined `git clone` step when the legacy source is remote — see the snippet in [`plugins/rt/skills/wiretapper/SKILL.md`](../../../plugins/rt/skills/wiretapper/SKILL.md)):

```text
git clone "$URL" "$DEST"   --> bootstrap the legacy repo (inlined snippet)
/rt:wiretapper             --> instrument it and capture fixtures
/rt:replay-writer          --> write regression tests from fixtures
```

This pipeline is typically used alongside the core Specify workflow:

1. Clone and wiretap the legacy service to capture fixtures.
2. Use `/change:plan` with `source legacy=<path>` to plan the migration.
3. Use `/change:execute` to implement each slice.
4. Use `/rt:replay-writer` to add regression tests that verify the new implementation matches the legacy behavior.
