## 0.37.0

Unreleased

### Compatibility

```text
engine 0.37.x  ↔  adapters 0.11.x  (WIT emery:adapter@0.1.0, floor ≥ 0.37.0)
```

### Changed

* `emery adapter update` is now `emery adapter upgrade` (HTTP `/adapter/upgrade`; error discriminants `adapter-upgrade-*`). Adds `emery adapter upgrade --all` to force a registry check for every bare binding in `project.yaml` / `plan.yaml` sources.
* Launcher refresh anchoring: relative `--project-dir` joins the walked project root (same base as the guest `.` mount), so `adapter upgrade --all` from a subdirectory still widens the refresh set.
* Text-mode `error:` lines render bold red on stderr (`NO_COLOR` / `TERM` respected); JSON output unchanged.
* RFC-82 (draft): cross-repo changesets design — forge-reconstructible membership, operator-owned publication, read-only verification surface.

**Full Changelog**: https://github.com/augentic/emery/compare/v0.36.0...v0.37.0

---

Release notes for previous releases can be found on the respective release branches of the repository.

<!-- ARCHIVE_START -->
* [0.37.x](https://github.com/augentic/emery/blob/release-0.37.0/RELEASES.md)
* [0.36.x](https://github.com/augentic/emery/blob/release-0.36.0/RELEASES.md)
* [0.35.x](https://github.com/augentic/emery/blob/release-0.35.0/RELEASES.md)
* [0.34.x](https://github.com/augentic/emery/blob/release-0.34.0/RELEASES.md)
* [0.33.x](https://github.com/augentic/emery/blob/release-0.33.0/RELEASES.md)
* [0.32.x](https://github.com/augentic/emery/blob/release-0.32.0/RELEASES.md)
* [0.31.x](https://github.com/augentic/emery/blob/release-0.31.0/RELEASES.md)
* [0.30.x](https://github.com/augentic/emery/blob/release-0.30.0/RELEASES.md)
* [0.28.x](https://github.com/augentic/emery/blob/release-0.28.0/RELEASES.md)
