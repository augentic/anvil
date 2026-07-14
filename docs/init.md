# `specify init`

`specify init` scaffolds the per-project `.specify/` tree plus
`project.yaml`. It has two mutually exclusive shapes; missing both
prompts for the adapter when stdin is a TTY and surfaces the typed
`init-adapter-required` (exit 2) everywhere else (CI, agents, pipes).

## Regular project — `specify init <adapter>`

Pass an adapter identifier or a directory/URL that resolves to one:

```bash
specify init omnia
specify init https://github.com/augentic/omnia.git
specify init ./path/to/adapter
```

The adapter supplies the schemas, plan template, and registry hooks
the project will use. The CLI writes:

- `project.yaml` (adapter identifier, `specify` floor, and
  `platforms` when the target declares a platform capability).
- `.specify/` (slices, archive, scratch, journal, guest.lock marker).
- `.specify/wasm-pkg.toml` — project-local wasm-pkg registry config,
  prefilled with the canonical `specify -> augentic.io` namespace
  mapping. Edit it to point first-party tool fetches at an internal
  mirror or to register additional namespaces. The file is checked
  in; re-running `init` never overwrites operator edits.

## Workspace — `specify init --workspace`

```bash
specify init --workspace --name <workspace-name>
```

A workspace is a registry-only project: it owns `registry.yaml` and
the cross-repo workspace slots, but does not itself host adapter artifacts.
Use this for the platform repo that orchestrates a fleet of adapter
projects. Workspace init writes `workspace: true` in `project.yaml`,
seeds an empty `registry.yaml`; workspace slot materialization remains operator-owned
before returning (no-op when `projects: []`, but still upserts
`.gitignore` and canonicalises an empty `topology.lock`). Workspace init
also writes `.specify/wasm-pkg.toml` so workspace operators can publish or
pull packages with `wkg --config .specify/wasm-pkg.toml` against the same
registry config the runtime honours.

## Why the two shapes are exclusive

An adapter project pins one adapter identifier; a workspace pins
none (it owns the registry of many). Mixing the two would produce a
`project.yaml` whose semantics depend on whether downstream verbs
treat the project as an adapter source or as a registry root, and
different verbs would disagree. Supplying both is a clap conflict
(exit 2); supplying neither fails typed with `init-adapter-required`
(exit 2) — with the `init-requires-adapter-or-workspace` discriminant
as the workflow layer's defence-in-depth check.

## Re-entry

Running `specify init` in an already-initialized project (one whose
`.specify/project.yaml` exists) changes nothing and exits 0 with a
message routing to `specify init --upgrade` — the re-entry flag that
bumps the `specify` pin, re-runs hydration over the declared set, and
preserves every operator artifact.
