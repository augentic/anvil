Implement the tasks in tasks.md. Do not write Rust or Swift code
directly -- delegate to the skills below.

Arguments (used by all skills):
- CHANGE_ID: the name of this change (from specify status)
- MODULE_NAME: the spec folder name (specs/<module>/spec.md)
- PROJECT_DIR: the target project directory
- IOS_SHELL_DIR: the root directory of the iOS shell project (e.g. `$PROJECT_DIR/ios`)
- APP_NAME: the Xcode target / Swift source folder name (e.g. `MyApp`)

## Module type detection

Check the proposal to determine which module types are in scope.
Process modules in this order:

1. **design-system** modules first (other modules may depend on tokens)
2. **core** modules next (iOS shell depends on the core)
3. **ios-shell** modules last

---

## Core modules

Check whether `{PROJECT_DIR}/shared/src/app.rs` exists:

- If `app.rs` does not exist, use create mode.
- If `app.rs` exists, use update mode.

### Create mode (app.rs does NOT exist -- new core)

#### Phase 1: Generate

1. /vectis:core-writer -- generate the Crux shared crate

Run the skill from start to finish. Complete every step in the skill's
process and ensure its verification checklist is satisfied before moving
to the next phase.

#### Phase 2: Verify and repair

Run the verify-repair loop described below.

#### Phase 3: Review

2. /vectis:core-reviewer -- AI code review

### Update mode (app.rs exists -- incremental change)

#### Step 0: Capture baseline

Before any changes, record the current test state:

```bash
cd $PROJECT_DIR && cargo test 2>&1 | tee /tmp/${CHANGE_ID}-${MODULE_NAME}-baseline.txt
```

Record which tests pass and which fail. This baseline is used in
Phase 2 to detect regressions.

#### Phase 1: Generate

1. /vectis:core-writer -- update the Crux shared crate

Run the skill from start to finish. Complete every step in the skill's
process and ensure its verification checklist is satisfied before moving
to the next phase.

#### Phase 2: Verify and repair

Run the verify-repair loop described below. In update mode, step 3
includes a regression check: compare post-test results against the
baseline captured in Step 0.

#### Phase 3: Review

2. /vectis:core-reviewer -- AI code review

---

## iOS shell modules

Check whether the iOS shell directory exists and contains `.swift` files:

- If no Swift files exist, use create mode.
- If Swift files exist, use update mode.

### Create mode (new iOS shell)

#### Phase 1: Generate

1. /vectis:ios-writer -- generate the iOS shell

#### Phase 2: Verify

Run the iOS verify steps described below.

#### Phase 3: Review

2. /vectis:ios-reviewer -- AI code review

### Update mode (existing iOS shell)

#### Phase 1: Generate

1. /vectis:ios-writer -- update the iOS shell

#### Phase 2: Verify

Run the iOS verify steps described below.

#### Phase 3: Review

2. /vectis:ios-reviewer -- AI code review

---

## Design system modules

1. /vectis:design-system-writer -- generate design system from tokens

No separate verify-repair loop needed; the skill includes its own
swift build verification step.

---

## Core verify-repair loop (max 3 iterations)

After core-writer has completed, run this loop to converge on a clean
build. Each iteration runs all three checks; if any fail, apply the
targeted fix and start a new iteration.

### 1. Formatting

```bash
cd $PROJECT_DIR && cargo fmt --check
```

If fails: run `cargo fmt` to fix.

### 2. Compilation and lint

```bash
cd $PROJECT_DIR && cargo check
cd $PROJECT_DIR && cargo clippy --all-targets
```

If fails: fix each error or warning.

### 3. Test suite

```bash
cd $PROJECT_DIR && cargo test
```

If failures are detected, classify each failure and route the fix to
core-writer for repair. Pass the full error output as context so the
skill can make a targeted fix.

### Repair discipline

When re-entering a skill for repair:

- **Minimum change only** -- fix the reported error and nothing else.
- **Scope the diff** -- before committing a repair, verify the change
  is limited to files and functions identified in the error output.

**Update mode only -- regression check**: compare post-test results
against the baseline from Step 0. For each test that passed before
and now fails:

- If the test asserts behavior that the **updated spec explicitly
  changes**, the failure is an **expected behavioral change**, not a
  regression.
- If the test asserts behavior that the spec does **not** change, the
  failure is a **true regression**.

### Loop control

Repeat from step 1 until all three checks pass or 3 iterations are
exhausted. If still failing after 3 iterations: **STOP**. Do not mark
the task complete. Report the remaining failures with full error output
and escalate for guidance.

---

## iOS verify steps

### 1. Format

```bash
swiftformat $IOS_SHELL_DIR/$APP_NAME/
```

### 2. Build

```bash
cd $IOS_SHELL_DIR && make build
```

If fails: fix the issue and re-run.

### 3. Simulator build

```bash
cd $IOS_SHELL_DIR && make sim-build
```

If fails: fix the issue and re-run.
