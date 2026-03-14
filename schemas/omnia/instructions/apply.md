Implement the tasks in tasks.md. Do not write Rust code directly --
delegate to the skills below.

Arguments (used by all skills):
- CHANGE_ID: the name of this change (from specify status)
- PROJECT_DIR: the project root (typically ".")
- CRATE_NAME: the spec folder name (specs/<crate>/spec.md)
- CRATE_PATH: PROJECT_DIR/crates/CRATE_NAME

## Mode detection

Check whether CRATE_PATH/Cargo.toml exists:

- If Cargo.toml does not exist, use create mode.
- If Cargo.toml exists, use update mode.

**Create mode** (Cargo.toml does NOT exist -- new crate):
  1. /omnia:guest-writer -- generate the WASM guest project
  2. /omnia:crate-writer -- generate the domain crate from Specify artifacts
  3. /omnia:test-writer -- generate comprehensive test suites from spec scenarios
  4. (Optional) /omnia:code-reviewer -- AI code review

**Update mode** (Cargo.toml exists -- incremental change):
  1. /omnia:crate-writer -- update the domain crate
  2. /omnia:test-writer -- update tests to match changed specs

Run each skill from start to finish. Do not implement manually.
Complete every step in each skill's process and ensure its
verification checklist is satisfied.
