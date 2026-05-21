# specify CLI commands (post RFC-25)

Specify 2.0 v1 target surface. Global on all rows: `--format text|json` (`SPECIFY_FORMAT`).

The v1 floor: the CLI is the single writer of files the skills must not hand-edit (`project.yaml`, `plan.yaml`, `.metadata.yaml`, archive paths), plus a small set of computations and side effects the agent shouldn't reimplement. Everything else — status, show, list, diagnostic helpers — is cut. Operators read YAML and Markdown files directly; skills do the same. Verbs return when a real caller asks for them.

| Command | Positionals | Flags |
|---------|-------------|-------|
| `specify init` | `<target>` | `--name`, `--domain` |
| `specify init` | | `--workspace`, `--name`, `--domain` |
| `specify source resolve` | `<name>` | `--project-dir` |
| `specify target resolve` | `<value>` | `--project-dir` |
| `specify plan create` | `<name>` | `--source` |
| `specify plan add` | `<name>` | `--depends-on`, `--sources`, `--description`, `--project`, `--target`, `--context` |
| `specify plan amend` | `<name>` | `--depends-on`, `--sources`, `--add-source`, `--remove-source`, `--description`, `--project`, `--target`, `--context`, `--divergence` |
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

`<target>` for `specify plan transition`: plan lifecycle `reviewed`; per-entry `done`. `pending` is written by `plan add` / `plan amend`, and `in-progress` is written only by `plan next`. `plan next` returns the active `in-progress` entry before selecting a new `pending` entry, and reports drained only when no active or pending entries remain. v1 has no per-entry `blocked`, `failed`, or `skipped` state; build failures and merge conflicts leave the active entry `in-progress`. `<target>` for `specify slice transition`: `refining`, `refined`, `built`, `dropped` (`--reason` only for `dropped`; the `merged` state is stamped by `specify slice merge`, never `slice transition`). Repeatable flags: `plan create --source`, `plan add` / `amend` `--depends-on` / `--sources` / `--add-source` / `--remove-source` / `--context`, `workspace prepare-branch` `--source` / `--output`. `plan add` / `plan amend` `--sources` and `--add-source` take `<key>=<candidate-id>` arguments — the source key references a top-level `plan.yaml.sources.<key>` binding and the candidate id references a `## Candidate inventory` block in `discovery.md`; the bare `<key>` shorthand is accepted only when the candidate id equals the slice's own `name` (typical for `intent`). `--remove-source` takes `<key>` alone (one binding per key per slice). `plan amend --add-source` / `--remove-source` only succeed while the slice's per-entry lifecycle is `pending` and the plan lifecycle is at most `reviewed`; rebinding an already-extracted slice requires `slice transition dropped` and re-add. `plan amend --divergence <accepted|rejected>` writes `slices[].divergence` and is accepted at any per-entry lifecycle state; the field is advisory metadata in v1 (no halt/park is wired against any value) and records operator acknowledgement (or rejection) of the `propose`-time `likely` prediction. `none` cannot be set explicitly — absence is none — and `likely` is reserved for the `propose` sub-step.

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

**Deferred — separate consumer ask.**

- `slice transition refined_provisional` — the second structural gate (operator review of synthesis output as a parking state). Multi-source synthesis ships in v1 (RFC-25); `/spec:refine` surfaces `[conflict]` / `[divergence]` / `[unknown]` inline in `spec.md` as review signals and `/spec:build` does not refuse on those tags. The `divergence:` enum on slice entries already carries the Gate-1 acknowledgement signal a future park would consume, with `surfaced` / `confirmed` / `resolved` reserved as forward-compatible values, so the parking state can be wired in without a schema change when a real consumer demands review-then-promote ergonomics, automation hooks, or CI gating around synthesis output.
- `--parallel-extract` flag (or implicit parallelism) on `/spec:refine`. v1 runs `extract` serially in `planSlice.sources` declaration order for deterministic goldens; parallel extraction returns when extract latency becomes a real workflow cost.
- `plan.yaml.slices[].authority-override` and per-claim authority overrides. v1 uses adapter-class defaults; per-slice and per-claim overrides return when editing `spec.md` after `[divergence]` is no longer an adequate operator seam.

**Operator-curated YAML — hand-edit, validation on first use.**

`specify registry add`, `specify registry remove`. `AGENTS.md` does not forbid hand-editing `registry.yaml` (the off-limits list is `.metadata.yaml`, archive paths, and `.specify/` scaffolding). Operators edit `registry.yaml` directly; `workspace sync` and `/spec:plan` validate at first use.

**Permanent surface for transient or never-existing need.**

- `specify upgrade` — migration ships as `migrate-to-2.0.sh` with the release notes.
- `specify plan archive` — covered by `plan finalize`.
- `specify plan lock {acquire, release, status}` — internal to `/spec:execute` and the breakout verbs.
- `specify tool gc` — `rm -rf .specify/.cache/` until cache pressure is a real workflow.
- `specify codex export` — moves into a `codex` target adapter under `specify target *`.

**Borderline — ship if trivial, otherwise defer.**

`specify completions <shell>` — no skill caller, but `clap_complete` is one line and shell completion is the most-expected nicety in a CLI. Ship when the `clap_complete` dependency is paid for any reason.

**Retired RFC-25 surface (pre-redesign verbs that never reach v1):**

`specify adapter *`, `specify change *`, `specify change survey`, `specify plan doctor`.

## When verbs come back

Add a verb when at least one of these is true:

1. A skill body is reimplementing nontrivial domain logic that should live in the CLI.
2. A documented external consumer (CI, hosted runner, third-party adapter) needs the structured shape.
3. The on-disk file the verb writes is documented as off-limits to hand-editing.

Speculation — "we might need this someday" — is not on the list.
