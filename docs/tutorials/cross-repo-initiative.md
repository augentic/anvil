# Cross-Repo Initiatives

When an initiative spans multiple repositories -- a backend, a mobile app, a shared library -- Specify coordinates planning and execution across all of them. This tutorial builds on the [single-repo initiative](single-repo-initiative.md) and introduces the registry, workspace sync, and project assignment.

**Prerequisites:** Familiarity with [multi-change initiatives](single-repo-initiative.md).

## The problem

A single-repo plan assumes all changes execute in one project. But real platforms often span multiple repos:

- A backend API repo (Omnia schema)
- A mobile app repo (Vectis schema)
- A shared types repo

Each change in the plan needs to run in the right repo, with the right schema. Without coordination, the operator must manually track which changes go where.

## 1. Create the registry

The **registry** is a YAML file that declares the repos in your platform. Create it at `.specify/registry.yaml`:

```yaml
version: 1
projects:
  - name: api
    url: git@github.com:org/api.git
    schema: omnia@v1
    description: >
      REST API and business logic. Owns authentication,
      order processing, and notification dispatch.

  - name: mobile
    url: git@github.com:org/mobile.git
    schema: vectis@v1
    description: >
      iOS and Android mobile application. Owns the
      client-side experience, offline sync, and push
      notification handling.
```

The `description` field is the key signal -- it tells Specify what each repo is responsible for, which drives change-to-project assignment.

> When the registry declares a single project, `description` is optional. With multiple projects, it is required.

Validate the registry:

```bash
specify initiative registry validate
```

## 2. Author the initiative brief (optional)

For complex initiatives, write an operator brief at `.specify/initiative.md`:

```bash
specify initiative brief init
```

This scaffolds the file. Edit it to describe your intent:

```markdown
---
name: add-oauth
inputs:
  - key: monolith
    path: /path/to/legacy
    kind: legacy-code
  - key: prd
    path: ./docs/oauth-prd.md
    kind: documentation
---

Add OAuth2 authentication across the platform. The API needs
provider integration and token management. The mobile app
needs login/registration screens and token refresh.
```

## 3. Plan across repos

Run the plan skill as usual, but now the registry is in play:

```text
/spec:plan add-oauth --source monolith=/path/to/legacy --from ./docs/oauth-prd.md
```

The plan skill runs its four-phase flow for multi-repo initiatives:

### Discovery

Same as single-repo -- analyses inputs and produces `discovery.md`.

### Sync peers

Because `registry.yaml` declares multiple projects, the **sync peers** phase runs automatically:

1. Clones every registry project into `.specify/workspace/<project>/`.
2. Inventories each repo's existing `.specify/` tree -- baseline specs, in-flight plans, schema.
3. Produces `workspace.md` with the peer inventory, including each project's `Description` and `Schema` from the registry.

You can review the result at `.specify/plans/add-oauth/workspace.md`.

### Propose

Same as single-repo -- the propose brief decomposes the capability inventory into change slices via the interactive accept / edit / reject loop. Entries are created **without** a `project` field; assignment happens in the next step.

### Assignment

After propose, the plan skill runs the **assignment pass**. For each newly created entry, it infers which project the change belongs to using three signals:

1. **Registry descriptions** -- the primary signal. Each change's description is matched against project descriptions in `workspace.md`.
2. **Baseline specs** -- capabilities already specified in a repo have strong affinity.
3. **Schema identity** -- a UI capability is unlikely to route to an `omnia` backend project.

The full assignment table is presented in a batch review:

```
## Assignment

| # | Entry | Project | Rationale |
|---|---|---|---|
| 1 | add-token-management | api | description overlap: authentication, token management |
| 2 | add-login-screens | mobile | schema: vectis (UI capability) |
```

You can override any assignment. Ambiguous entries are surfaced as unresolved and require your input. Each assignment is written via `specify plan amend <name> --project <project>`.

## 4. Review the cross-repo plan

After planning, the plan has a `project` field on each entry:

```yaml
changes:
  - name: add-token-management
    description: "OAuth2 token management and provider integration"
    project: api
    depends-on: []
    status: pending

  - name: add-login-screens
    description: "Login and registration screens with OAuth flow"
    project: mobile
    depends-on: [add-token-management]
    status: pending
```

The `project` field tells `/spec:execute` where to run each change.

## 5. Execute across repos

```text
/spec:execute --loop
```

For each change, the driver uses **CWD-based routing** (RFC-3b):

1. Reads the `project` field from `specify plan next`.
2. Resolves source paths to absolute paths (anchored to the initiating repo).
3. Changes working directory to the target project's workspace clone.
4. Runs `/spec:define`, `/spec:build`, `/spec:merge` — phase skills are unaware of multi-repo routing.
5. Restores CWD to the initiating repo for the next iteration.

Changes execute in dependency order. `add-token-management` (api) runs first, then `add-login-screens` (mobile) can start because its dependency is `done`.

## 6. Push results

After execution, workspace clones contain local commits from merge. Push them to remotes:

```text
specify workspace push
```

This creates a `specify/<initiative-name>` branch per project, pushes to the remote, and opens a PR. For greenfield projects, it also creates the remote repo via `gh`. See [specify workspace](../reference/cli/workspace.md) for details.

## 7. Greenfield variant

For new platforms where repos do not exist yet, the registry describes the *intended* organisation. Use git remote URLs so that `workspace sync` can bootstrap clones and `workspace push` can push to the remote:

```yaml
projects:
  - name: api
    url: .
    schema: omnia@v1
    description: >
      REST API and business logic.

  - name: mobile
    url: git@github.com:org/mobile.git
    schema: vectis@v1
    description: >
      iOS and Android mobile application.
```

When `specify workspace sync` encounters a remote that does not exist, it treats the project as greenfield: creates the workspace slot, runs `git init`, sets the remote, and bootstraps `.specify/project.yaml` via `specify init` using the initiating repo's schema cache. After execution, `specify workspace push` creates the remote repo (via `gh`) and pushes.

## What you learned

- `registry.yaml` declares the repos in your platform with domain descriptions.
- `/spec:plan` automatically syncs peers when the registry has multiple projects.
- Propose creates entries without `project`; the assignment step infers and writes project routing.
- `plan.yaml` entries carry a `project` field for CWD-based routing during execution.
- `/spec:execute` routes each change to the correct workspace clone and schema.
- `specify workspace push` publishes local commits to remotes and opens PRs.
- The same flow works for brownfield (existing repos) and greenfield (new repos).

## Next

[Legacy Migration at Scale](legacy-migration-at-scale.md) -- decompose a large monolith across multiple target repos using the analyze/extract split.
