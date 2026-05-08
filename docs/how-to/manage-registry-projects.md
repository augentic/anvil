# Manage Registry Projects

`registry.yaml` is the platform catalogue declaring which repos are in scope for cross-repo changes. Projects are added and removed via `specify registry add` and `specify registry remove` (RFC-9 Section 2A) -- never by hand-editing `registry.yaml` and hoping validation passes.

This how-to covers the day-to-day flow: registering a new project, removing one, handling the `description-missing-multi-repo` invariant, and rewiring plan entries after a removal.

## Prerequisites

- A bootstrapped hub or platform-as-project repo with a populated `.specify/`. See [Bootstrap a platform hub](bootstrap-a-platform-hub.md).
- The new project's remote URL (or local path -- see [URL classification](#url-classification) below).
- A capability identifier for the new project (`omnia@v1`, `vectis@v1`, or any other resolvable capability).

## Add a project

```bash
specify registry add <name> \
    --url <url> \
    --schema <capability> \
    --description "<one-paragraph domain description>"
```

The verb runs four checks before any write:

1. **Name validation:** `<name>` must match `^[a-z][a-z0-9-]*$` (kebab-case).
2. **URL classification:** the URL must be one of `.`, a repo-relative path (`../foo`), an absolute path (`/abs/path`), `git@host:path`, `http(s)://...`, `ssh://...`, or `git+http(s)://` / `git+ssh://`.
3. **Capability non-empty:** the `--schema` capability value must be non-empty after trim.
4. **Existing-name collision:** the verb refuses to add a project that already exists -- update via a hand-edit + `specify registry validate` instead, or remove and re-add.

After the write, `Registry::validate_shape` runs -- including the `description-missing-multi-repo` invariant.

### URL classification

| URL shape | Workspace materialisation |
|-----------|---------------------------|
| `.` | Symlink to the initiating repo (only valid in platform-as-project mode; rejected on hubs with `hub-cannot-be-project`). |
| `../foo`, `/abs/path` | Symlink to the resolved local path. `specify workspace push` reads `git remote get-url origin` from the local repo to find the push target. |
| `git@host:path`, `http(s)://...`, `ssh://...` | Shallow `git clone` into `.specify/workspace/<name>/`. |

### Greenfield projects (remote does not exist yet)

Register the project anyway -- `specify workspace sync` and `specify workspace push` will bootstrap the remote on demand. The greenfield path runs `git init` in the workspace slot, sets the remote, and (on the next `workspace push`) runs `gh repo create` if the URL is a GitHub remote.

## Remove a project

```bash
specify registry remove <name>
```

The verb refuses when the registry is absent or the named project is not declared. After the write, `validate_shape` runs against the resulting registry to catch any cascading invariant breakage.

A non-fatal warning fires when `plan.yaml` exists and any plan entry references the removed project. The warning names each affected entry so you can rewire them via `specify change plan amend <change> --project <other>` separately. Until you rewire them, the affected entries will refuse to advance through the executor (the `project-not-in-registry` validator code blocks them).

## Handle `description-missing-multi-repo`

`registry add` refuses when the addition produces a multi-project registry and any **existing** entry lacks a `description`. The invariant fires at the moment a registry crosses from one project to two:

```text
description-missing-multi-repo: registry.yaml: projects[<idx>] (<name>) has no `description`
```

There is no in-band fix because `registry add` cannot retro-add a description to an existing entry without overwriting it. Two recovery paths:

1. **Hand-edit `registry.yaml`** to add the missing description, then re-run `specify registry add` for the new project.
2. **Recreate the offending entry**: `specify registry remove <existing>`, then `specify registry add <existing> --url <existing-url> --schema <existing-capability> --description "..."`. (This second path also triggers the plan-reference warning if the existing entry was already wired into a plan -- preferable to use the hand-edit unless the description rewrite is the intent.)

The descriptions matter beyond validation: `/change:plan`'s assignment step reads them when inferring which project each plan entry targets. Sparse descriptions force unresolved (`?`) prompts during planning. Plan a paragraph per project, not a sentence.

## After registry mutation: re-sync

Any `add` or `remove` should be followed by `specify workspace sync` to bring the workspace clones in line with the registry. The verb is idempotent -- existing slots refresh, missing slots materialise.

```bash
specify registry add new-project --url ... --schema <capability> --description "..."
specify workspace sync
```

For ongoing changes where a plan entry was already authored against the now-removed project, the validator will block `/change:execute` until you run `specify change plan amend <entry> --project <other>` (or `specify change plan transition <entry> skipped`).

## Validation ordering invariant

The plan verbs reject unknown projects: `specify change plan add --project <name>` and `specify change plan amend --project <name>` both validate `<name>` against `registry.yaml`. The implication is a strict ordering:

```text
specify registry add <name> ...     # before any plan write that references <name>
specify workspace sync              # bootstrap the workspace slot
specify change plan add ... --project <name>   # OR specify change plan amend ... --project <name>
```

`/change:plan`'s registry-proposal sub-step (RFC-9 Section 2B) enforces the same order automatically. Hand-driven flows must respect it manually.

## See also

- [`specify registry`](../reference/cli/registry.md) -- full CLI reference for `add` / `remove` / `show` / `validate`.
- [Bootstrap a platform hub](bootstrap-a-platform-hub.md) -- the first time you populate a registry.
- [Cross-Repo Changes](../tutorials/cross-repo-change.md) -- how registry descriptions feed into `/change:plan`'s assignment.
- [Recover from registry-amendment-required](recover-from-registry-amendment.md) -- handling the case where `/change:execute` halts and proposes a new project.
- [`description-missing-multi-repo`](../appendices/troubleshooting.md#description-missing-multi-repo) -- troubleshooting entry.
