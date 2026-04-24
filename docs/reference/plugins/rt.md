# RT Plugin

Fixture capture, repository cloning, and regression testing for migrations. The RT plugin supports the migration workflow by providing tools to clone legacy repositories, capture runtime fixtures, and write regression tests from those fixtures.

## Skills

### /rt:git-cloner

Clone a source repository for analysis.

**Synopsis:**

```text
/rt:git-cloner <repo-url> <dest-dir> [--detach]
```

**Inputs:**
- `repo-url` -- Git repository URL.
- `dest-dir` -- Local destination directory.
- `--detach` -- Remove `.git` directory after cloning (for analysis without git history).

**Behavior:**
1. Validates the URL and destination.
2. Clones (shallow) or pulls if the destination already exists.
3. Verifies the clone.
4. Reports summary (language, file count, size).

Used by `/spec:plan` during the discovery phase when `--source` points to a remote repository.

### /rt:wiretapper

Capture request/response and side-effect data from a legacy TypeScript service.

**Synopsis:**

```text
/rt:wiretapper <legacy-dir> [--app-name <name>]
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
/rt:replay-writer <crate-name> [--project-dir <path>]
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

The three RT skills form a pipeline:

```text
/rt:git-cloner     --> clone the legacy repo
/rt:wiretapper     --> instrument it and capture fixtures
/rt:replay-writer  --> write regression tests from fixtures
```

This pipeline is typically used alongside the core Specify workflow:

1. Clone and wiretap the legacy service to capture fixtures.
2. Use `/spec:plan` with `--source legacy=<path>` to plan the migration.
3. Use `/spec:execute` to implement each change.
4. Use `/rt:replay-writer` to add regression tests that verify the new implementation matches the legacy behavior.
