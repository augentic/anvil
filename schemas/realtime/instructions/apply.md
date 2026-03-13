Implement the tasks in tasks.md. Do not write Rust code directly --
delegate to the skills below.

Arguments (used by all skills):
- CHANGE_ID: the name of this change (from specify status)
- PROJECT_DIR: the project root (typically ".")
- CRATE_NAME: the spec folder name (specs/<crate>/spec.md)
- CRATE_PATH: PROJECT_DIR/crates/CRATE_NAME

Mode detection -- check whether CRATE_PATH/Cargo.toml exists:
- If Cargo.toml does not exist, use create mode.
- If Cargo.toml exists, use update mode.

**Create mode** (Cargo.toml does NOT exist -- new crate):
  1. Use the /omnia:guest-writer skill to generate the WASM guest project 
    (src/lib.rs, Cargo.toml, Provider). Run the skill from start to finish;
    do not implement the guest manually. Complete every step in the skill,
    and ensure the skill's verification checklist is satisfied.

  2. Use the /omnia:crate-writer skill to generate the domain crate
    (types, handlers, baseline tests) from Specify artifacts. 
    Run the skill from start to finish; do not implement the crate
    manually. Complete every step in the skill, and ensure the skill's
    verification checklist is satisfied.

  3. Use the /omnia:test-writer skill to generate comprehensive test suites
    from spec scenarios. Run the skill from start to finish; do not implement
    the tests manually. Complete every step in the skill, and ensure the skill's
    verification checklist is satisfied.
  
  4. Verify from CRATE_PATH (repair loop -- max 3 iterations):
       a. cargo fmt --check
          If fails: run cargo fmt, then re-check. Formatting is
          mechanical; one pass should suffice.
       b. cargo clippy -- -D warnings
          If fails: fix each warning using repair-patterns.md
          (clippy section), then re-run clippy.
       c. cargo test
          If fails: analyze each failure and apply the matching
          repair strategy from repair-patterns.md:
          - Type mismatches: update struct definitions, serde attrs
          - Validation errors: fix from_input() or handle() logic
          - Provider mock errors: update MockProvider trait bounds
          - Logic errors: compare handler against artifacts
          Then re-run cargo test.
       Repeat from (a) until all three pass or 3 iterations are
       exhausted. If still failing after 3 iterations: STOP. Do NOT
       mark the task complete. Report the remaining failures with
       error output and ask for guidance.
  
  5. (Optional) /omnia:code-reviewer <CRATE_PATH>
     AI code review for security, WASM constraints, and quality.

**Update mode** (Cargo.toml exists -- incremental change):
  1. Use the /omnia:crate-writer skill to update the domain crate. Run the
     skill from start to finish; do not implement the crate manually.
     Complete every step in the skill, and ensure the skill's verification
     checklist is satisfied.

  2. Use the /omnia:test-writer skill to regenerate or update tests to match
     changed specs. Run the skill from start to finish; do not implement the
     tests manually. Complete every step in the skill, and ensure the skill's
     verification checklist is satisfied.
     Regenerates or updates tests to match changed specs.
     
  3. Verify from CRATE_PATH (repair loop -- max 3 iterations):
       a. cargo fmt --check
          If fails: run cargo fmt, then re-check.
       b. cargo clippy -- -D warnings
          If fails: fix each warning using repair-patterns.md, re-run.
       c. cargo test
          If fails: analyze each failure, apply the matching repair
          strategy (type mismatch, validation error, provider mock
          error, logic error), re-run.
       Repeat from (a) until all three pass or 3 iterations are
       exhausted. If still failing after 3 iterations: STOP. Do NOT
       mark the task complete. Report failures and ask for guidance.

Read context files, work through pending tasks, mark complete as you go.
Pause if you hit blockers or need clarification.
