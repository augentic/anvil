---
id: build
description: Implement the tasks in tasks.md by delegating to the skills below
needs: [specs, design, tasks]
tracks: tasks
---

Arguments (used by all skills):
- CHANGE_ID: the name of this change (from specify status)
- FEATURE_NAME: the spec folder name (specs/<feature>/spec.md)
- PROJECT_DIR: the target project directory
- IOS_SHELL_DIR: the root directory of the iOS shell project (e.g. `$PROJECT_DIR/iOS`)
- ANDROID_SHELL_DIR: the root directory of the Android shell project (e.g. `$PROJECT_DIR/Android`)
- APP_NAME: the Xcode target / Swift source folder name (e.g. `MyApp`)

## Platform detection

Read the proposal to determine which platforms are in scope.
Process platforms in this dependency order:

1. **design-system** first (other platforms may depend on tokens)
2. **core** next (shells depend on the core)
3. **ios** and **android** shells (independent of each other)
4. **web** shell (future)

### Parallel shell generation

iOS and Android shells have no dependencies on each other -- both
depend on core + design-system only. When both platforms are in scope,
spawn their **generation** sub-agents (Phase 1) concurrently after core
verify-repair completes. Pass `skip_verification: true` to each writer
so they produce code without invoking build tools (ios-writer skips
step 11; android-writer skips step 15):

```
core verify-repair done
    ├── spawn: ios-writer sub-agent   (skip_verification: true)
    └── spawn: android-writer sub-agent (skip_verification: true)
         (both run in parallel)
```

Wait for both generation sub-agents to complete, then run **verify**
phases serially and **review** phases in parallel. Because the writers
skipped their internal verification, the orchestrator's verify
sub-agents are the sole point of build verification.

**Why verify is serial:** Both iOS verify (`make build` → cargo-swift)
and Android verify (`make build` → uniffi typegen, `gradlew
:shared:cargoBuild`) invoke `cargo` against the same shared Rust
workspace. Cargo uses a workspace-level lock file, so concurrent
invocations serialize on the lock rather than running in parallel. Keep
verify serial to avoid the overhead of lock contention and redundant
process management.

**Why review is parallel:** The review phases are pure code-analysis
agent teams (3 specialists + 1 antagonist). The iOS reviewer reads and
auto-fixes files exclusively under `iOS/` (plus read-only access to
`shared/src/app.rs` and `design-system/`); the Android reviewer does
the same under `Android/`. They use different formatting tools
(`swiftformat` vs Kotlin formatter) and never invoke `cargo`, Gradle,
or Xcode. With no shared mutable state and no build-tool contention,
they are safe to run concurrently without degrading review accuracy.

```
both writers done
    → iOS verify sub-agent       (serial -- cargo workspace lock)
    → Android verify sub-agent   (serial -- cargo workspace lock)
both verified
    ├── iOS review sub-agent     (parallel -- code analysis only)
    └── Android review sub-agent (parallel -- code analysis only)
both reviews done
    → consolidate design-level findings (see below)
```

When both reviews complete, the orchestrator consolidates design-level
findings from both platforms into a single Specify change rather than
letting each reviewer call `/spec:define` independently. This avoids
concurrent writes to `.specify/changes/` and produces a unified
cross-platform review change. See **Consolidate design-level findings**
below.

If only one shell platform is in scope, still pass
`skip_verification: true` and run the dedicated verify sub-agent
afterward -- this keeps the contract consistent regardless of how
many platforms are active.

Each skill reads the single feature spec at
`specs/<feature>/spec.md`. The spec contains core requirements in
the main body and platform-specific requirements in dedicated
sections (e.g. `## iOS Shell Requirements`).

---

## Agent environment constraints

Several verification commands depend on network access, external
toolchains, or long-running processes that may not behave reliably in
all agent environments. Follow these rules during verification:

**Network failures:** Commands like `gradle`, `./gradlew`, and
`swift build` (package resolution) may fail with connection resets,
SSL errors, or timeouts in sandboxed environments. If a command fails
with network-related errors, log the full error, do **not** retry
indefinitely, mark the task as blocked on environment, and report to
the user. A single retry is acceptable; repeated retries of the same
network error are not.

**Timeouts:** Use generous timeouts for commands that resolve
dependencies or compile large projects. Recommended minimums:
- Gradle configure / build: 120 seconds
- `swift build`: 60 seconds
- `cargo test`: 60 seconds

If a command is backgrounded, poll for completion and read the output
rather than assuming success or failure from the timeout alone.

**Missing prerequisites:** If `./gradlew` is not executable or
`gradle/wrapper/gradle-wrapper.jar` is missing, follow the wrapper
bootstrap procedure in the Android verify section below rather than
re-running the broken command. If `gradle` itself is not installed,
report the prerequisite and mark verification as pending.

**Stuck sessions:** If verification output stops and no progress is
visible after polling, kill the process and report the last output
rather than waiting indefinitely. Avoid backgrounding multiple cargo
processes against the same workspace concurrently -- they contend for
the workspace-level lock file. Gradle and Xcode use separate
toolchains and do not contend with each other, but avoid multiple
Gradle daemons or multiple Xcode builds in the same project
simultaneously.

---

## Sub-agent delegation

Each `/vectis:*` skill invocation and each verify-repair loop runs in
its **own sub-agent** with a clean context window. The orchestrator
(this document) coordinates the sequence but does not execute skill
steps inline.

### Why sub-agents

A full greenfield build loads design-system, core, test, iOS, and
Android skills sequentially. Without delegation, the orchestrator's
context accumulates thousands of lines of skill instructions, reference
material, generated code, and compiler output that are irrelevant to
later phases. Sub-agents start fresh, carrying only the material needed
for their specific task.

### Delegation pattern

For each skill invocation:

1. **Spawn** a sub-agent with a prompt containing:
   - The skill name to invoke (e.g., `/vectis:core-writer`)
   - The standard arguments (CHANGE_ID, FEATURE_NAME, PROJECT_DIR, etc.)
   - The mode (create or update) already determined by the orchestrator
   - Any phase-specific context (e.g., error output for repair, baseline
     test log for update-mode verification)

2. **Wait** for the sub-agent to complete and read its result.

3. **Assess** the result before proceeding:
   - If the sub-agent reports success, continue to the next phase
   - If the sub-agent reports failure with actionable errors, spawn a
     repair sub-agent (see verify-repair sections) or escalate
   - If the sub-agent reports `pending` (e.g., environment blocker),
     log the reason and continue to the next platform

The sub-agent reads the skill's SKILL.md and references itself -- the
orchestrator does not need to pre-read or relay skill instructions.

### Handoff contract

Each sub-agent receives and returns structured information:

**Inputs (orchestrator -> sub-agent):**

| Field | Description |
| --- | --- |
| `skill` | Skill name (e.g., `vectis:core-writer`) |
| `arguments` | Standard arguments: CHANGE_ID, FEATURE_NAME, PROJECT_DIR, shell dirs, APP_NAME |
| `mode` | `create`, `update`, or `repair` (determined by orchestrator before spawning) |
| `skip_verification` | If `true`, the skill skips its internal build-verification step (ios-writer step 11 / U8, android-writer step 15 / U8). The orchestrator sets this for shell writers and runs verification in a dedicated sub-agent afterward. Defaults to `false` for standalone invocations. |
| `artifact_paths` | Paths to spec, design, and proposal files |
| `orchestrated` | Reviewer sub-agents only. If `true`, the reviewer returns `design_findings` in its output and skips step 3 (`/spec:define`). The orchestrator consolidates findings across platforms. Defaults to `false` for standalone invocations, where the reviewer creates its own Specify change. |
| `extra_context` | Phase-specific: error output for repair, baseline test log for regression checks, prior phase warnings |

**Outputs (sub-agent -> orchestrator):**

| Field | Description |
| --- | --- |
| `status` | `success`, `failure`, or `pending` |
| `files_modified` | List of paths created or changed |
| `verification` | Internal verification result if the skill includes one |
| `errors` | Error details when status is `failure` or `pending` |
| `warnings` | Non-blocking issues for downstream phases |
| `design_findings` | Reviewer sub-agents only: accumulated design-level findings with classification (`code-fix` or `spec-change`), check IDs, severity, and suggested fixes. Empty list when no design-level findings exist. The orchestrator consolidates these across platforms rather than letting each reviewer create its own Specify change. |

### Verify-repair sub-agents

Verify-repair loops also run as sub-agents. When a verify sub-agent
needs to re-enter a skill for repair, it spawns a **nested repair
sub-agent** for the targeted fix rather than re-reading the entire
skill in its own context. The nested repair sub-agent receives only:

- The skill name (`vectis:core-writer` or `vectis:test-writer`)
- The full error output to fix
- The repair discipline constraints (minimum change, scoped diff,
  one failure class per re-entry)
- The mode: `repair` (not `create` or `update`)

This keeps each verification iteration lightweight -- the verify
sub-agent holds only the classification table, build commands, and
iteration state, not the full 3,000+ line skill context.

---

## Core

Check whether `{PROJECT_DIR}/shared/src/app.rs` exists:

- If `app.rs` does not exist, use create mode.
- If `app.rs` exists, use update mode.

The core-writer reads the main body of the feature spec (core
requirements) and the design.md Domain Model and Capabilities sections.
Platform-specific sections in the spec are not relevant to core
generation.

### Create mode (app.rs does NOT exist -- new core)

#### Phase 1: Generate code

1. /vectis:core-writer -- generate the Crux shared crate

Spawn a sub-agent to run the skill. Pass standard arguments with
`mode: create`. The sub-agent reads the skill's SKILL.md and
references, completes every step, and returns its verification
checklist result. Wait for completion before proceeding.

#### Phase 2: Generate tests

2. /vectis:test-writer -- generate spec-traced tests

Spawn a sub-agent to run the skill. It generates the `#[cfg(test)]`
module in `app.rs` with one test per spec scenario and traceability
comments linking each test to its `REQ-XXX` ID.

#### Phase 3: Verify and repair

Spawn a sub-agent to run the core verify-repair loop described below.
Pass PROJECT_DIR and the feature spec path.

#### Phase 4: Review

3. /vectis:core-reviewer -- AI code review

Spawn a sub-agent to run the skill. The reviewer internally creates
its own agent team (3 specialists + antagonist per agent-teams.md).

### Update mode (app.rs exists -- incremental change)

#### Step 0: Capture baseline

Before spawning any sub-agents, record the current test state:

```bash
cd $PROJECT_DIR && cargo test 2>&1 | tee /tmp/${CHANGE_ID}-${FEATURE_NAME}-baseline.txt
```

Record which tests pass and which fail. This baseline is passed to
the verify-repair sub-agent in Phase 3 for regression detection.

#### Phase 1: Generate code

1. /vectis:core-writer -- update the Crux shared crate

Spawn a sub-agent to run the skill. Pass standard arguments with
`mode: update`. The sub-agent reads the skill's SKILL.md and
references, completes every step, and returns its verification
checklist result. Wait for completion before proceeding.

#### Phase 2: Generate/update tests

2. /vectis:test-writer -- update spec-traced tests

Spawn a sub-agent to run the skill. It diffs spec scenarios against
existing tests, adds tests for new scenarios, updates tests for
modified scenarios, and flags stale tests for removed scenarios.

#### Phase 3: Verify and repair

Spawn a sub-agent to run the core verify-repair loop described below.
Pass PROJECT_DIR, the feature spec path, and the baseline test log
from Step 0 as `extra_context` for regression checking.

#### Phase 4: Review

3. /vectis:core-reviewer -- AI code review

Spawn a sub-agent to run the skill. The reviewer internally creates
its own agent team (3 specialists + antagonist per agent-teams.md).

---

## iOS shell

Only run this section if `ios` is listed in the proposal's Platforms.

The ios-writer reads `app.rs` as its primary input, supplemented by
the `## iOS Shell Requirements` section of the feature spec and the
`## iOS Shell Details` section of design.md.

Check whether the iOS shell directory exists and contains `.swift` files:

- If no Swift files exist, use create mode.
- If Swift files exist, use update mode.

### Create mode (new iOS shell)

#### Phase 1: Generate

1. /vectis:ios-writer -- generate the iOS shell

Spawn a sub-agent to run the skill with `mode: create` and
`skip_verification: true`. Pass standard arguments including
IOS_SHELL_DIR and APP_NAME. The writer generates all code but does
not run step 11 (format and verify) -- that is handled by the
dedicated verify sub-agent in Phase 2.

#### Phase 2: Verify

Spawn a sub-agent to run the iOS verify steps described below. Pass
IOS_SHELL_DIR and APP_NAME.

#### Phase 3: Review

2. /vectis:ios-reviewer -- AI code review

Spawn a sub-agent to run the skill with `orchestrated: true`. The
reviewer runs its review-fix cycle (steps 1-2) and returns
`design_findings` in its output instead of calling `/spec:define`
directly. The orchestrator consolidates these findings across
platforms after both reviews complete. The reviewer internally
creates its own agent team (3 specialists + antagonist per
agent-teams.md).

### Update mode (existing iOS shell)

#### Phase 1: Generate

1. /vectis:ios-writer -- update the iOS shell

Spawn a sub-agent to run the skill with `mode: update` and
`skip_verification: true`. Pass standard arguments including
IOS_SHELL_DIR and APP_NAME. The writer applies changes but does not
run step U8 (format and verify) -- that is handled by the dedicated
verify sub-agent in Phase 2.

#### Phase 2: Verify

Spawn a sub-agent to run the iOS verify steps described below. Pass
IOS_SHELL_DIR and APP_NAME.

#### Phase 3: Review

2. /vectis:ios-reviewer -- AI code review

Spawn a sub-agent to run the skill with `orchestrated: true`. The
reviewer runs its review-fix cycle (steps 1-2) and returns
`design_findings` in its output instead of calling `/spec:define`
directly. The orchestrator consolidates these findings across
platforms after both reviews complete. The reviewer internally
creates its own agent team (3 specialists + antagonist per
agent-teams.md).

---

## Android shell

Only run this section if `android` is listed in the proposal's Platforms.

The android-writer reads `app.rs` as its primary input, supplemented by
the `## Android Shell Requirements` section of the feature spec and the
`## Android Shell Details` section of design.md.

Check whether the Android shell directory exists and contains `.kt` files:

- If no Kotlin files exist, use create mode.
- If Kotlin files exist, use update mode.

### Create mode (new Android shell)

#### Phase 1: Generate

1. /vectis:android-writer -- generate the Android shell

Spawn a sub-agent to run the skill with `mode: create` and
`skip_verification: true`. Pass standard arguments including
ANDROID_SHELL_DIR. The writer generates all code but does not run
step 15 (build and verify) -- that is handled by the dedicated verify
sub-agent in Phase 2.

#### Phase 2: Verify

Spawn a sub-agent to run the Android verify steps described below.
Pass ANDROID_SHELL_DIR.

#### Phase 3: Review

2. /vectis:android-reviewer -- AI code review

Spawn a sub-agent to run the skill with `orchestrated: true`. The
reviewer runs its review-fix cycle (steps 1-2) and returns
`design_findings` in its output instead of calling `/spec:define`
directly. The orchestrator consolidates these findings across
platforms after both reviews complete. The reviewer internally
creates its own agent team (3 specialists + antagonist per
agent-teams.md).

### Update mode (existing Android shell)

#### Phase 1: Generate

1. /vectis:android-writer -- update the Android shell

Spawn a sub-agent to run the skill with `mode: update` and
`skip_verification: true`. Pass standard arguments including
ANDROID_SHELL_DIR. The writer applies changes but does not run
step U8 (build and verify) -- that is handled by the dedicated verify
sub-agent in Phase 2.

#### Phase 2: Verify

Spawn a sub-agent to run the Android verify steps described below.
Pass ANDROID_SHELL_DIR.

#### Phase 3: Review

2. /vectis:android-reviewer -- AI code review

Spawn a sub-agent to run the skill with `orchestrated: true`. The
reviewer runs its review-fix cycle (steps 1-2) and returns
`design_findings` in its output instead of calling `/spec:define`
directly. The orchestrator consolidates these findings across
platforms after both reviews complete. The reviewer internally
creates its own agent team (3 specialists + antagonist per
agent-teams.md).

---

## Design system

Only run this section if `design-system` is listed in the proposal's
Platforms.

The design-system-writer reads `tokens.yaml` as its primary input,
supplemented by the `## Design System Requirements` section of the
feature spec if present.

1. /vectis:design-system-writer -- regenerate iOS Swift Package and Android
   `vectis-design` library from tokens

Spawn a sub-agent to run the skill. The skill includes `swift build`
for the iOS package and `./gradlew :vectis-design:compileDebugKotlin`
for the Android module once the shell's `settings.gradle.kts` includes
`:vectis-design`. Verification is internal to the skill.

---

## Consolidate design-level findings

After both shell reviews complete (or the single review when only one
platform is in scope), the orchestrator collects the `design_findings`
output from each reviewer sub-agent and decides whether to create a
Specify change.

1. **Merge findings.** Combine `design_findings` from the iOS reviewer
   and Android reviewer into a single list. Deduplicate any universal
   findings (UNI-prefixed) that both reviewers flagged with identical
   check IDs and matching evidence -- keep the higher-severity instance.
   Platform-specific findings (IOS-, SWF-, AND-, KTL-, INT-prefixed)
   are always distinct.

2. **Check if any findings exist.** If the merged list is empty, skip
   the rest of this section.

3. **Classify each finding** as `code-fix` or `spec-change` using the
   criteria from the reviewer skills (spec is clear but code is wrong →
   code-fix; spec is silent, ambiguous, or problematic → spec-change).

4. **Derive a change name.** When findings span both platforms, use:

   ```
   review-{app-name}-shells-{YYYY-MM-DDTHH-MM}
   ```

   When findings come from only one platform, use the platform-specific
   name (`review-{app-name}-ios-...` or `review-{app-name}-android-...`).

   ```bash
   date -u +"%Y-%m-%dT%H-%M"
   ```

5. **Delegate to `/spec:define`** with the derived change name and a
   description synthesized from all accumulated findings. Content
   guidelines:

   - **proposal.md**: Summarize findings by platform and severity. The
     "What Changes" section lists each finding as a bullet, prefixed
     with `[spec]` or `[code]` and `[ios]` / `[android]` / `[both]` to
     indicate scope. Note mechanical fixes already applied per platform
     and how many review cycles each ran.

   - **design.md**: Group related findings across platforms where
     applicable (e.g., the same missing effect handler surfaces in both
     iOS and Android reviews). Reference the platform-specific check IDs
     that motivated each decision.

   - **specs/**: Create one spec file per logical area. When a finding
     applies to both platforms, write a single requirement with
     platform-specific acceptance criteria sections rather than
     duplicating the requirement.

   - **tasks.md**: Order tasks: spec updates first, then core changes
     (if any), then iOS fixes, then Android fixes, then verification.
     Include a final task per platform that re-runs the corresponding
     reviewer skill to confirm all Critical findings are resolved.

6. **Show final status** using `/spec:status`.

---

## Core verify-repair loop (max 3 iterations)

This loop runs in its **own sub-agent**, separate from the core-writer
and test-writer sub-agents that preceded it. The orchestrator spawns
this sub-agent with PROJECT_DIR, the spec path, and (in update mode)
the baseline test log. The sub-agent runs the checks below and returns
`status`, `iterations_used`, and any unresolved errors.

Each iteration runs all three checks; if any fail, apply the targeted
fix and start a new iteration.

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
the appropriate skill via a **nested repair sub-agent**:

| Failure signal | Classification | Fix action |
| --- | --- | --- |
| Error in `#[cfg(test)] mod tests`, test helper functions, or factory functions | **Test issue** | Spawn repair sub-agent for test-writer with the error output |
| Error in production code (`app.rs` outside `#[cfg(test)]`), missing types or methods | **Code issue** | Spawn repair sub-agent for core-writer with the error output |
| Assertion mismatch where the *actual* value looks correct per spec | **Test issue** | Spawn repair sub-agent for test-writer -- the expected value is wrong |
| Assertion mismatch where the *expected* value matches spec | **Code issue** | Spawn repair sub-agent for core-writer -- the handler returns the wrong result |
| Type mismatch between handler output and test assertion | **Code issue** if handler type is wrong per spec; **test issue** if assertion type is stale | Classify per spec, spawn the appropriate repair sub-agent |
| Unresolved import or missing crate in `Cargo.toml` | **Workspace issue** | Fix `Cargo.toml` directly (no sub-agent needed) |

Each repair sub-agent receives only: the skill name, the full error
output, the repair discipline constraints below, and `mode: repair`.
It does **not** re-read the full skill references -- just enough
context to make a targeted fix.

### Repair discipline

Repair sub-agents follow these constraints:

- **Minimum change only** -- fix the reported error and nothing else.
- **Scope the diff** -- before committing a repair, verify the change
  is limited to files and functions identified in the error output.
- **One failure class per sub-agent** -- if multiple failures are
  present, group them by classification (code issue vs test issue) and
  spawn one repair sub-agent per class with all same-class errors. Do
  not interleave code and test fixes in a single sub-agent.

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

## iOS verify steps (max 3 iterations)

This loop runs in its **own sub-agent**, separate from the ios-writer
sub-agent that preceded it. The orchestrator spawns this sub-agent with
IOS_SHELL_DIR and APP_NAME. The sub-agent runs the checks below and
returns `status`, `iterations_used`, and any unresolved errors.

### 1. Format

```bash
swiftformat $IOS_SHELL_DIR/$APP_NAME/
```

### 2. Build

```bash
cd $IOS_SHELL_DIR && make build
```

If fails: fix the issue and re-run from step 1.

### 3. Simulator build

```bash
cd $IOS_SHELL_DIR && make sim-build
```

If fails: fix the issue and re-run from step 1.

### Loop control

Repeat from step 1 until all three checks pass or **3 iterations** are
exhausted. If the same error recurs across iterations with no change in
output, stop early. If still failing after 3 iterations: **STOP**. Do not
mark the task complete. Report the remaining failures with full error output
and escalate for guidance.

---

## Android verify steps

This loop runs in its **own sub-agent**, separate from the
android-writer sub-agent that preceded it. The orchestrator spawns this
sub-agent with ANDROID_SHELL_DIR. The sub-agent runs the checks below
and returns `status`, `iterations_used`, and any unresolved errors.

### 0. Pre-flight checks

Before entering the verify loop, validate project configuration. These
checks do not invoke build tools and should fail fast on
misconfigurations:

1. Verify `local.properties` has `sdk.dir` set.
2. Verify `gradle.properties` has `org.gradle.java.home` pointing to
   Java 21.
3. Verify Rust Android targets are installed:
   `rustup target list --installed | grep android`

If any check fails, report the missing prerequisite and mark Android
verification as **pending** rather than entering the build loop.

### Gradle wrapper bootstrap

Before running any `./gradlew` command, verify the wrapper is usable:
`gradlew` must exist, be executable, and `gradle/wrapper/gradle-wrapper.jar`
must be present. If the wrapper is missing or incomplete, bootstrap it from a
minimal init project (no AGP, no `settings.gradle.kts` includes):

```bash
tmp_dir=$(mktemp -d)
cd "$tmp_dir" && gradle wrapper && cd -
cp "$tmp_dir/gradlew" "$tmp_dir/gradlew.bat" "$ANDROID_SHELL_DIR/"
cp -r "$tmp_dir/gradle" "$ANDROID_SHELL_DIR/"
chmod +x "$ANDROID_SHELL_DIR/gradlew"
rm -rf "$tmp_dir"
```

If `gradle` is not installed, report the prerequisite error
(`brew install gradle`) and mark Android verification as **pending**.

### 1. Type generation

```bash
cd $ANDROID_SHELL_DIR && make build
```

If fails: fix the issue and re-run.

### 2. Rust library build

```bash
cd $ANDROID_SHELL_DIR && ./gradlew :shared:cargoBuild
```

If fails: fix the issue and re-run.

### 3. APK build

```bash
cd $ANDROID_SHELL_DIR && ./gradlew :app:assembleDebug
```

If fails: fix the issue and re-run.

### Loop control

Repeat from step 1 until all three checks pass or **3 iterations** are
exhausted. If the same error recurs across iterations with no change in
output, stop early. If still failing after 3 iterations: **STOP**. Do not
mark the task complete. Report the remaining failures with full error output
and escalate for guidance.
