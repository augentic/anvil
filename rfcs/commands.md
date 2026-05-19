# specify CLI commands (post RFC-25 + RFC-26)

Specify 3.0 v1 target surface. Global on all rows: `--format text|json` (`SPECIFY_FORMAT`).

The v1 floor: the CLI is the single writer of files the skills must not hand-edit (`project.yaml`, `plan.yaml`, `.metadata.yaml`, archive paths), plus a small set of computations and side effects the agent shouldn't reimplement. Everything else — status, show, list, diagnostic helpers — is cut. Operators read YAML and Markdown files directly; skills do the same. Verbs return when a real caller asks for them.

| Command | Positionals | Flags |
|---------|-------------|-------|
| `specify init` | `<target>` | `--name`, `--domain` |
| `specify init` | | `--hub`, `--name`, `--domain` |
| `specify source resolve` | `<name>` | `--project-dir` |
| `specify target resolve` | `<value>` | `--project-dir` |
| `specify plan create` | `<name>` | `--source` |
| `specify plan add` | `<name>` | `--depends-on`, `--sources`, `--description`, `--project`, `--target`, `--context` |
| `specify plan amend` | `<name>` | `--depends-on`, `--sources`, `--description`, `--project`, `--target`, `--context` |
| `specify plan transition` | `<name>`, `<target>` | `--reason` |
| `specify plan next` | | |
| `specify plan finalize` | `<name>` | `--clean`, `--dry-run` |
| `specify slice create` | `<name>` | `--target`, `--if-exists` |
| `specify slice transition` | `<name>`, `<target>` | `--reason` |
| `specify slice validate` | `<name>` | |
| `specify slice merge` | `<name>` | `--dry-run`, `--check-only` |
| `specify workspace sync` | `[<project>…]` | |
| `specify workspace push` | `[<project>…]` | `--dry-run` |
| `specify workspace prepare-branch` | `<project>` | `--change`, `--source`, `--output` |
| `specify tool run` | `<name>`, `[args…]` | arguments after `--` |

`<target>` for `specify plan transition`: plan lifecycle `reviewed`; per-entry `pending`, `in-progress`, `done`, `blocked`, `failed`, `skipped` (`--reason` only for `failed`, `blocked`, `skipped`). `<target>` for `specify slice transition`: `defining`, `defined`, `built`, `dropped` (`--reason` only for `dropped`; the `merged` state is stamped by `specify slice merge`, never `slice transition`). Repeatable flags: `plan create --source`, `plan add` / `amend` `--depends-on` / `--sources` / `--context`, `workspace prepare-branch` `--source` / `--output`.

## What was cut and why

**Reads — operator or agent opens the file directly.**

`specify status`, `specify plan show`, `specify plan status`, `specify slice status`, `specify workspace status`, `specify registry show`, `specify source list`, `specify target list`, `specify tool list`, `specify tool show`, `specify slice journal show`, `specify slice outcome show`, `specify slice task progress`. Every one of these formatted a YAML or Markdown file that anyone can `cat`. No skill needs the CLI to read state back to it; the agent reads `.specify/project.yaml`, `.specify/plan.yaml`, `.specify/slices/<name>/.metadata.yaml` directly.

**Validation folded into the write verb.**

- `specify plan validate` — `plan add` and `plan amend` refuse to write an invalid plan; first-use validation is the seam.
- `specify source validate`, `specify target validate` — `source resolve` and `target resolve` validate the manifest on load.
- `specify registry validate` — `workspace sync` and `/spec:plan` refuse to operate on a malformed registry.
- `specify context check` — not needed without `context generate`.

**Folded into a parent verb.**

- `specify slice drop` → `specify slice transition <name> dropped --reason "..."`.
- `specify slice outcome set` — not needed in v1. Slice lifecycle alone tells `/spec:execute` where to resume; the chat session and on-disk artifacts carry the failure diagnostic. Persisted phase outcomes are observability and belong with RFC-19. Reinstate when crash-recovery diagnostics must survive an agent restart.
- `specify slice journal append` — defer to RFC-19; nothing in v1 signals through the journal.
- `specify context generate` — `specify init` writes the initial `AGENTS.md` and `.specify/context.lock`. Drift detection (`--check`) is a CI affordance; ship when a CI integration asks for it.
- `specify tool fetch` — `specify tool run` fetches `.wasm` on first call.

**No skill caller in v1 — topology and helpers hand-coded in skills until a real caller appears.**

- `specify slice synthesize`, `specify target build`, `specify target merge` — synthesis and target brief topology for the two or three known target adapters (omnia, vectis, contracts) is hand-coded in `/spec:refine`, `/spec:build`, `/spec:merge`. Reinstate when a third-party target ships with custom brief ordering.
- `specify slice touched-specs` — `/spec:merge` diffs the slice's `specs/` against the baseline inline.
- `specify slice overlap` — parallel-slice safety; single-operator v1 has no parallel slices to coordinate.
- `specify slice task progress`, `specify slice task mark` — `/spec:build` greps `- [ ]` in `tasks.md` and edits the checkbox in place.
- `specify compatibility check` — defer until a real cross-project consumer exists.

**Deferred with the multi-source extension (RFC-25 §Non-Goals).**

- `specify plan amend --add-source <key>` / `--remove-source <key>` — slice rebinding is only meaningful when a slice can carry more than one source. v1 binds one source at `specify plan add` and that binding is the slice's. Reinstate together with the rest of the multi-source surface (authority hierarchy, `[divergence]` tag, inter-pack `[conflict]` detection, parallel extract).
- `slice transition defined_provisional` — the second structural gate (operator review of synthesis output as a parking state) ships with the multi-source extension. v1 surfaces `[conflict]` / `[unknown]` inline in `spec.md` and relies on operator hand-edit before `/spec:build`.

**Operator-curated YAML — hand-edit, validation on first use.**

`specify registry add`, `specify registry remove`. `AGENTS.md` does not forbid hand-editing `registry.yaml` (the off-limits list is `.metadata.yaml`, archive paths, and `.specify/` scaffolding). Operators edit `registry.yaml` directly; `workspace sync` and `/spec:plan` validate at first use.

**Permanent surface for transient or never-existing need.**

- `specify upgrade` — migration ships as `migrate-to-3.0.sh` with the release notes.
- `specify plan archive` — covered by `plan finalize`.
- `specify plan lock {acquire, release, status}` — internal to `/spec:execute` and the breakout verbs.
- `specify tool gc` — `rm -rf .specify/.cache/` until cache pressure is a real workflow.
- `specify codex export` — moves into a `codex` target adapter under `specify target *`.

**Borderline — ship if trivial, otherwise defer.**

`specify completions <shell>` — no skill caller, but `clap_complete` is one line and shell completion is the most-expected nicety in a CLI. Ship when the `clap_complete` dependency is paid for any reason.

**Retired RFC-25/26 surface (pre-redesign verbs that never reach v1):**

`specify adapter *`, `specify change *`, `specify change survey`, `specify plan doctor`.

## When verbs come back

Add a verb when at least one of these is true:

1. A skill body is reimplementing nontrivial domain logic that should live in the CLI.
2. A documented external consumer (CI, hosted runner, third-party adapter) needs the structured shape.
3. The on-disk file the verb writes is documented as off-limits to hand-editing.

Speculation — "we might need this someday" — is not on the list.
