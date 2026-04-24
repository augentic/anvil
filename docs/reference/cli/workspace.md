# specify workspace

Materialise, inspect, and push workspace peer clones for multi-repo initiatives.

## Subcommands

### specify workspace sync

Clone or refresh every project declared in `.specify/registry.yaml` into `.specify/workspace/<project>/`.

```bash
specify workspace sync
```

For each registry project:

- **Remote URL** (`git@`, `ssh://`, `https://`, `http://`) -- shallow-clones the repo into the workspace slot.
- **Local path** (`.`, `../foo`, `/absolute/path`) -- symlinks the resolved path into the workspace slot.
- **Greenfield** (remote URL, repo does not yet exist) -- creates the workspace slot, runs `git init`, sets the remote, and bootstraps `.specify/project.yaml` via `specify init <schema> --schema-dir <dir>` using the initiating repo's `.specify/.cache/`.

A partially bootstrapped slot (`.git/` present but `.specify/project.yaml` absent) is detected on re-run: `specify init` is re-attempted without re-running `git init` or `git remote add`.

Non-zero exit if any project fails, with a per-project status summary.

### specify workspace status

Report the materialisation state of every registry project's workspace slot.

```bash
specify workspace status
```

Per-project output includes: slot path, materialisation type (`symlink`, `git-clone`, `missing`), HEAD sha, dirty flag, and `.specify/` tree summary.

### specify workspace push

Push workspace clones that have local commits back to their remote repositories.

```bash
specify workspace push [<project>...]
```

Omitting the project argument pushes all dirty clones. The initiative name for branch naming (`specify/<initiative-name>`) is read from `.specify/plan.yaml`.

**Per-project algorithm:**

1. **Remote resolution.** Remote URLs are used directly. Local paths read `git remote get-url origin`; if no remote exists, the project is skipped with `local-only` status.
2. **Branch.** Creates or updates `specify/<initiative-name>` from the clone's current HEAD.
3. **Repo creation (greenfield).** If the remote does not exist and the URL is a GitHub URL, creates the repo via `gh repo create`.
4. **Push.** `git push --force-with-lease -u origin specify/<initiative-name>`.
5. **PR.** Creates a PR via `gh pr create` if none exists for the branch.

**Flags:**

| Flag | Description |
|------|-------------|
| `--dry-run` | Classify each project's push status without performing any writes. No `git push`, `gh repo create`, or `gh pr create`. |
| `--format json` | Machine-readable JSON output. |

**Output (human-readable):**

```text
specify: workspace push — <initiative-name>

  traffic        pushed       specify/platform-v2  PR #42
  command-centre up-to-date
  mobile         created      specify/platform-v2  PR #7

1 created, 1 pushed, 1 up-to-date. 0 failed.
```

**Status vocabulary:** `created` (remote repo created, greenfield), `pushed` (existing remote updated), `up-to-date` (no local commits ahead), `local-only` (no remote configured), `failed` (error).

**Prerequisites:** `gh` (GitHub CLI) is required only when repo creation or PR creation is needed. Plain `git push` works for any forge.
