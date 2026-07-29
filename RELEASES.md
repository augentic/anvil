## 0.32.0

Released 2026-07-29

### Compatibility

```text
engine 0.32.x  ↔  adapters 0.6.x  (WIT emery:adapter@0.1.0, floor ≥ 0.30.0)
```

### Added

### Changed

* Publish Release builds platform archives (`x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`) as workflow artifacts first, then attaches them when the GitHub Release is created — fixing empty or missing binary attachments from the previous post-tag binaries job.
* Release docs and the audit workflow follow the same in-workflow binaries path; the standalone `binaries.yaml` workflow is removed.
* Quick-start install examples drop the Homebrew block for now and pin `cargo binstall` / `cargo install --tag` examples to a released host version.

**Full Changelog**: https://github.com/augentic/emery/compare/v0.31.0...v0.32.0

---

Release notes for previous releases can be found on the respective release branches of the repository.

<!-- ARCHIVE_START -->
* [0.32.x](https://github.com/augentic/emery/blob/release-0.32.0/RELEASES.md)
* [0.31.x](https://github.com/augentic/emery/blob/release-0.31.0/RELEASES.md)
* [0.30.x](https://github.com/augentic/emery/blob/release-0.30.0/RELEASES.md)
* [0.28.x](https://github.com/augentic/emery/blob/release-0.28.0/RELEASES.md)
