# password-reset — tasks.md

- [ ] Author crate `crates/password_reset` per `design.md` (domain types, operation delegation, error mapping, provider bounds).
- [ ] Author tests under `crates/password_reset/tests/` covering each scenario in `spec.md` plus the publish-fails-gateway path.
- [ ] (First build only) Generate the workspace `src/lib.rs` with the `POST /password-reset` Axum route wired to `ResetRequest`.
- [ ] Run the verify-repair loop (`cargo fmt`, `cargo check`, `cargo clippy -- -D warnings`, `cargo test`).
- [ ] Run code review (`omnia-code-reviewer` body in `targets/omnia/briefs/build.md` §Code reviewer) and process findings.
