## 0.34.0

Released 2026-07-31

### Compatibility

```text
engine 0.34.x  ↔  adapters 0.8.x  (WIT emery:adapter@0.1.0, floor ≥ 0.34.0)
```

Hard cut for operator surface: `emery plan approve` is gone; Gate 1 is the first `emery plan execute`. Update skills, scripts, and docs that still call approve or the old merge/journal subcommands.

### Added

* `emery plan author --force` — replace a replaceable pending plan (`lifecycle: pending` and every entry `pending`); otherwise `already-exists` / `plan-author-not-replaceable`.
* Merge re-entry healing when commit/archive succeeded but the plan entry never reached `done`; `plan execute` auto-dispatches merge on `merge-incomplete` stops.

### Changed

* **Gate 1 is `emery plan execute`.** The standalone `emery plan approve` verb is removed. First execute on a `pending` plan stamps `approved` (idempotent), journals `plan.transition.approved` with `--actor` (default `operator`), then runs the drained loop. `/emery:execute` wraps the same verb behind confirmation; `/emery:plan` exits at `pending` and prints the literal execute command.
* `emery plan transition` is `--undo` only — forward `done` is owned solely by `emery slice merge`.
* Journal drops `emit`; every journal write remains an orchestration side effect. `emery journal show` is unchanged.
* `emery slice merge run` absorbs `--preview` and `--conflict-check` (dedicated merge subcommands removed from CLI/HTTP).
* `emery plan status` no longer stops on `plan-not-approved`; pending plans still project the next phase with `resume: /emery:execute`.
* Quick start and skill wrappers use bare adapter names and plain `--intent` strings instead of pinned `intent=…:value:…` plan bindings.

**Full Changelog**: https://github.com/augentic/emery/compare/v0.33.0...v0.34.0

---

Release notes for previous releases can be found on the respective release branches of the repository.

<!-- ARCHIVE_START -->
* [0.34.x](https://github.com/augentic/emery/blob/release-0.34.0/RELEASES.md)
* [0.33.x](https://github.com/augentic/emery/blob/release-0.33.0/RELEASES.md)
* [0.32.x](https://github.com/augentic/emery/blob/release-0.32.0/RELEASES.md)
* [0.31.x](https://github.com/augentic/emery/blob/release-0.31.0/RELEASES.md)
* [0.30.x](https://github.com/augentic/emery/blob/release-0.30.0/RELEASES.md)
* [0.28.x](https://github.com/augentic/emery/blob/release-0.28.0/RELEASES.md)
