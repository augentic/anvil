## 0.33.0

Released 2026-07-30

### Compatibility

```text
engine 0.33.x  ↔  adapters 0.7.x  (WIT emery:adapter@0.1.0, floor ≥ 0.33.0)
```

Hard cut: hosts and adapters from earlier lines do not speak the RFC-78 path-first seam. Re-init or re-pin both sides together.

### Added

* `scripts/install.sh` — `curl | sh` installer that downloads the GitHub Release archive for the detected platform, verifies the companion `.sha256`, and installs to `~/.local/bin` (`--version`, `--target`, `--dir`, `--dry-run`). `/emery:init` and the install docs recommend this path over `cargo binstall`.
* Adapter MCP over HTTP in the shipped runtime: the launcher pre-binds an HTTP listener and routes judgment grants to `/mcp/<axis>/<name>[@<version>]` on each adapter guest's `wasi:http` shelf.
* First-party **adapter train** recommendation: bare first-party names at init / `plan author` auto-pin to `emery:<name>@0.7.0` when the project cache misses; the pin is persisted and shown on `emery --version` as `(adapters <train>)`.
* Merge postflight failure handling: failed postflight reports are written beside the archive; `plan execute` / `plan status` stop with `merge-postflight-failed` until a re-run of `emery plan execute` acknowledges and continues.

### Changed

* **RFC-78 prompt budget (seam hard cut).** Target build inputs use `payload { path, body }` instead of inlined file bodies; `target.build` carries `build-context` (bound source names). Synthesis evidence moves to project-relative paths (`SYNTHESIS_VERSION` 2) so lent-workspace judgment legs read from the tree instead of duplicating content in prompts.
* Live eval / lab Cursor backend configuration renames `EVAL_MODEL` / `EVAL_TIMEOUT_SECS` to `CURSOR_MODEL` / `CURSOR_TIMEOUT_SECS`; backend default timeout documented as 600s when unset.
* RFC program realignment: deployment stays under RFC-71; RFC-70 is the migration walking-skeleton draft; RFC-78/79/80 added for prompt budget, swarm build, and synthesis redesign.

**Full Changelog**: https://github.com/augentic/emery/compare/v0.32.0...v0.33.0

---

Release notes for previous releases can be found on the respective release branches of the repository.

<!-- ARCHIVE_START -->
* [0.33.x](https://github.com/augentic/emery/blob/release-0.33.0/RELEASES.md)
* [0.32.x](https://github.com/augentic/emery/blob/release-0.32.0/RELEASES.md)
* [0.31.x](https://github.com/augentic/emery/blob/release-0.31.0/RELEASES.md)
* [0.30.x](https://github.com/augentic/emery/blob/release-0.30.0/RELEASES.md)
* [0.28.x](https://github.com/augentic/emery/blob/release-0.28.0/RELEASES.md)
