# specify context

Generate and check short, deterministic repository guidance for agents.

## Synopsis

```bash
specify context generate [--check] [--force]
specify context check
```

## Description

`specify context generate` writes a fenced `AGENTS.md` block at the repository root. The content is deterministic and derived from Specify project metadata, adapter briefs, registry data, active slice metadata, shallow root-marker detection, and the running CLI version.

`specify context check` compares the current renderer inputs and fenced body against `.specify/context.lock`. It exits non-zero when `AGENTS.md` is missing, the lock is missing, renderer inputs drifted, or generated fenced content was edited by hand.

`specify init` calls the same generator after successful project initialization when root `AGENTS.md` is absent. Existing `AGENTS.md` files are preserved.

## Generated files

| File | Owner | Purpose |
|------|-------|---------|
| `AGENTS.md` | Generated fenced block plus optional operator prose | Repository guidance for agents: runtime, tests, linting, navigation, conventions, boundaries, and dependencies. |
| `.specify/context.lock` | CLI-managed | Fingerprint sidecar for `specify context check`. |

Hub projects omit the `Runtime`, `Tests`, and `Linting` sections because hubs do not carry source code.

## Options

| Option | Description |
|--------|-------------|
| `--check` | Dry-run `generate`; exit 1 if `AGENTS.md` or `.specify/context.lock` would change. |
| `--force` | Rewrite when the existing `AGENTS.md` is unfenced, or when generated fenced content was modified. |
| `--format` | Global output format: `json` for structured automation output. |

## Fence policy

`generate` manages only the fenced block:

```markdown
<!-- specify:context begin
fingerprint: sha256:...
generated-by: specify ...
-->

...

<!-- specify:context end -->
```

When `AGENTS.md` already contains valid context fences, generation replaces only the managed block and preserves bytes before and after it. When `AGENTS.md` exists without fences, generation refuses with `context-existing-unfenced-agents-md` unless `--force` is passed. When the lock says the fenced body was hand-edited, generation refuses with `context-fenced-content-modified` unless `--force` is passed.

## Exit codes

| Command | Exit | Meaning |
|---------|------|---------|
| `generate` | 0 | Files were written or already current. |
| `generate --check` | 0 | Re-running generation would be a no-op. |
| `generate --check` | 1 | `AGENTS.md` or `.specify/context.lock` would change. |
| `check` | 0 | Context is up to date. |
| `check` | 1 | Context is missing, lock is missing, inputs drifted, or fences were modified. |
| `check` | 2 | Invocation or malformed-lock validation error. |

## JSON output

`generate --format json` returns the usual CLI envelope fields plus:

- `status` -- `written`, `unchanged`, or `would-update`
- `path` -- `AGENTS.md`
- `check` -- whether `--check` was used
- `force` -- whether `--force` was used
- `changed` -- whether any generated file changed or would change
- `agents-changed` -- whether `AGENTS.md` changed or would change
- `lock-changed` -- whether `.specify/context.lock` changed or would change
- `disposition` -- write policy outcome (`create`, `replace-fenced-block`, `force-rewrite-unfenced`, or `unchanged`)

`check --format json` returns:

- `status` -- `up-to-date`, `drift`, `context-not-generated`, or `context-lock-missing`
- `fingerprint.expected` and `fingerprint.actual`
- `inputs-changed`
- `inputs-added`
- `inputs-removed`
- `fences-modified`

## See also

- [`specify init`](init.md) -- generates starter context when initializing a project.
- [Directory Layout](../directory-layout.md) -- where generated context and lock files live.
