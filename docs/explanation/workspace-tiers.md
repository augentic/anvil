# Workspace Tiers

Specify materialises two different kinds of repository clones inside a `.specify/` tree, and they are not interchangeable. **Tier 1** is the legacy-source clone that `/spec:analyze` reads at plan time to build a capability inventory. **Tier 2** is the registered-project clone that `/change:execute` writes into during define-build-merge. Operator-facing language often calls both "workspace clones," which obscures their very different lifecycles and write semantics. This page makes the distinction explicit so the failure modes -- losing tier-1 writes, or mistaking tier-2 clones for ephemeral scratch space -- become obvious.

## The two tiers

| Tier | Location | Lifecycle | Writability |
|------|----------|-----------|-------------|
| Legacy-source clone | `.specify/plans/<name>/analyze/<key>/` | Ephemeral; swept by `specify change plan archive` | Read-only (analyze-only) |
| Registered project clone | `.specify/workspace/<name>/` | Durable; persists across changes | Read-write during execution; pushed by `specify workspace push` |

Each cell in the table maps to a different role:

- **Legacy-source clone (tier 1).** The directory under `.specify/plans/<name>/analyze/<key>/` belongs to a single change. It is populated by `/spec:analyze` (using the inlined guarded `git clone` snippet documented at [`../../plugins/spec/skills/analyze/SKILL.md` §*Cloning a source tree*](../../plugins/spec/skills/analyze/SKILL.md) when the source is a git URL) so the discovery brief can read source code that is not on the operator's local disk -- a remote monolith referenced by `--source <key>=<git-url>`, for example. The skill walks the tree, emits capability summaries into `.specify/plans/<name>/discovery.md`, and writes structural metadata at `.specify/plans/<name>/analyze/<key>/metadata.json`. Nothing else writes here. When `specify change plan archive` finalises the change, the entire `.specify/plans/<name>/` directory -- legacy-source clone included -- moves into `.specify/archive/plans/<YYYYMMDD>-<name>/` for audit.
- **Registered project clone (tier 2).** The directory under `.specify/workspace/<name>/` belongs to the **platform**, not to any one change. It is materialised by `specify workspace sync` from an entry in `registry.yaml` -- a shallow `git clone` for remote URLs, a symlink for relative or local paths, or a greenfield `git init` + `specify init` bootstrap when the remote does not yet exist. `/change:execute` prepares `specify/<change-name>`, then `chdir`s into this clone before invoking `/spec:define`, `/spec:build`, and `/spec:merge`, so the slice directory, merged baseline specs, residue commits, and workspace git history all accumulate here. `specify workspace push` is the explicit transport gate that publishes those local commits as a `specify/<change-name>` branch on the project's remote and opens a PR. It does not create branches on the fly, create commits, push default branches, or merge PRs. Clones persist across changes -- the next change against the same registry simply refreshes them with another `specify workspace sync`.

## Materialisation

Each tier is created by a different command, at a different stage of the loop.

| Tier | Materialised by | Triggered from |
|------|-----------------|----------------|
| Legacy-source clone | `/spec:analyze` (inlined `git clone` snippet for git URLs — see [its SKILL.md §*Cloning a source tree*](../../plugins/spec/skills/analyze/SKILL.md)) | The plan-time discovery brief, invoked by `/change:plan` step 3(a) |
| Registered project clone | `specify workspace sync` | `/change:plan` step 3(b) (sync-peers, multi-repo only); operators may also run it directly between runs |

Tier 1 is on-demand and per-input: discovery clones one tree per `--source <key>=<git-url>` (and per `kind: legacy-code` entry in `change.md:inputs`). Local paths and `--from` documentation inputs do not produce a tier-1 clone -- the skill reads them in place.

Tier 2 is registry-shaped: one slot per `projects[]` entry in `registry.yaml`. `specify workspace sync` is idempotent. Running it again refreshes existing slots (`git fetch` for remotes, no-op for symlinks) and bootstraps any slots that went missing. In single-repo changes the registry is absent or has at most one project, and tier 2 does not materialise at all -- everything happens in the initiating repo's working tree.

## Lifecycle

Each verb in the v1 CLI affects exactly one tier.

| Verb | Tier touched | Effect |
|------|--------------|--------|
| `/spec:analyze` (writes via the skill) | Tier 1 | Populates `.specify/plans/<name>/analyze/<key>/` and appends to `discovery.md` |
| `specify change plan archive` | Tier 1 | Sweeps `.specify/plans/<name>/` (legacy-source clones included) into `.specify/archive/plans/<YYYYMMDD>-<name>/` |
| `specify workspace sync` | Tier 2 | Materialises or refreshes `.specify/workspace/<name>/` for every registry project |
| `specify workspace status` | Tier 2 | Reports per-slot materialisation state (slot path, type, HEAD sha, dirty flag, `.specify/` tree summary) |
| `specify workspace push` | Tier 2 | Pushes already-prepared `specify/<change-name>` branches to remotes and creates or updates PRs |

No verb crosses tiers. There is no `workspace push` for legacy-source clones (they have no remote of interest), and no `plan archive` for the workspace cache (registered project clones outlive the change). The two trees also live under different parents in `.specify/`:

```text
.specify/
├── plans/
│   └── <change-name>/
│       ├── discovery.md
│       └── analyze/
│           └── <source-key>/    # tier 1 lives here
└── workspace/
    └── <project-name>/          # tier 2 lives here
```

```d2
direction: right

tier1: "Tier 1 — legacy-source clone" {
  shape: rectangle
  loc: ".specify/plans/<name>/analyze/<key>/" {shape: cylinder}
  analyze: "/spec:analyze" {shape: rectangle}
  archive: "specify change plan archive" {shape: rectangle}
  analyze -> loc: "writes (one per --source <k>=<git-url>)"
  loc -> archive: "swept into\n.specify/archive/plans/"
}

tier2: "Tier 2 — registered project clone" {
  shape: rectangle
  loc: ".specify/workspace/<project>/" {shape: cylinder}
  sync: "specify workspace sync" {shape: rectangle}
  execute: "/change:execute --loop" {shape: rectangle}
  push: "specify workspace push" {shape: rectangle}
  sync -> loc: "materialises (one per registry entry)"
  execute -> loc: "chdir + writes (define-build-merge)"
  loc -> push: "git push specify/<change-name>"
}
```

The arrows make the lifecycle explicit: tier 1 only flows from `/spec:analyze` to the archive, never back; tier 2 cycles between `sync` (refresh), `execute` (write), and `push` (release).

## Why the tiers are not interchangeable

The two tiers look superficially similar -- both contain working trees that Specify materialises on the operator's behalf -- but treating them as a single "workspace" leads to two specific failure modes:

- **Writes to tier 1 are lost when the change archives.** `specify change plan archive` is the closure verb for `.specify/plans/<name>/`. Anything an operator (or a misbehaving skill) edits inside `.specify/plans/<name>/analyze/<key>/` moves into the archive when the change ends -- it never propagates back to the original source. Treating a tier-1 clone as a place to make changes silently turns those changes into archived audit material. Tier 1 is read-only by design: the only outputs that escape it are the capability summaries and structural metadata that `/spec:analyze` writes alongside.
- **Tier 2 is the only place generated code lives across changes.** When `/change:execute` runs define-build-merge against a registered project, the slice directory, merged baseline, and residue commits accumulate inside `.specify/workspace/<project>/` -- not inside `.specify/plans/<name>/`. `specify workspace push` publishes those commits to the project's remote as a PR; subsequent changes pick them up via the next `specify workspace sync`. If an operator expects registered project clones to disappear at archive time, the durable history of the platform's evolution -- baselines, slice records, contract artefacts -- vanishes with them. Tier 2 is durable by design: the workspace cache is the staging area between successive changes.

The shorthand: **tier 1 is the input the planner reads; tier 2 is the output the executor writes.** The lifecycles match those roles, and the tooling enforces them.

## See also

- [Cross-Repo Changes](../tutorials/cross-repo-change.md) -- end-to-end worked example that exercises tier-2 materialisation, CWD routing during execution, and `specify workspace push`.
- [The Layered Stack](three-layer-stack.md) -- where `/change:plan` (default + `--orchestrate` modes) and `/change:execute` sit in the layered model.
- [`/change:plan`](../../plugins/change/skills/plan/SKILL.md) -- the skill that triggers tier-1 materialisation (step 3(a)) and tier-2 materialisation (step 3(b)).
- [`/change:execute`](../../plugins/change/skills/execute/SKILL.md) -- the skill that `chdir`s into tier-2 clones to drive define-build-merge.
- [`specify workspace`](../reference/cli/workspace.md) -- CLI reference for `sync`, `status`, and `push`.
- [`specify change plan`](../reference/cli/plan.md) -- CLI reference for `archive` (the tier-1 sweep).
