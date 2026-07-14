# greeting - Agent Instructions

<!-- specify:context begin
fingerprint: sha256:4ff77c641c13cee0c42a2c253ab0f0985b4a1d8069842426a185f04449bc4402
generated-by: specify 0.27.2
-->

## Runtime
- not detected

## Tests
- not detected

## Linting
- not detected

## Navigation
- `.specify/archive/` contains merged or dropped slice history.
- `.specify/project.yaml` stores Specify project metadata.
- `.specify/slices/` contains active slice workspaces.
- `change.md` is the repo-root change brief.
- `plan.yaml` is the optional repo-root platform plan.
- `registry.yaml` is the optional repo-root platform registry.
- active slices: 0 in `.specify/slices/`.

## Conventions
- adapter `fixture` 0.0.0.

## Boundaries
- During execute/build/merge, agents consume Specify and adapters — they do not maintain them.
- On scaffold, verify, finalize, or toolchain failure: stop, print CLI `stop:` / `hint:` / `resume:` output, and exit; never patch `specify`, `specify-adapters`, templates, `adapter.wasm`, or `~/.cache/specify/**` in-band.
- `.specify/archive/` is framework-managed history.
- `metadata.yaml` files are framework-managed; update them through `specify slice` commands.
- `plan.yaml` is framework-managed; write entries through `specify plan add` / `amend`, lifecycle through `specify plan transition`, and close-out through `specify plan archive` — never hand-edit it.
- `project.yaml` is the source of truth for Specify project metadata.
- adapter `fixture` owns generated artifact layout.

## Dependencies
- single-repo project; no registered peers.

<!-- specify:context end -->
