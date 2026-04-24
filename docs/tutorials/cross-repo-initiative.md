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

The plan skill runs its three-phase flow, but with an extra step:

### Discovery

Same as single-repo -- analyses inputs and produces `discovery.md`.

### Sync peers

Because `registry.yaml` declares multiple projects, the **sync peers** phase runs automatically:

1. Clones every registry project into `.specify/workspace/<project>/`.
2. Inventories each repo's existing `.specify/` tree -- baseline specs, in-flight plans, schema.
3. Produces `workspace.md` with the peer inventory.

You can review the result at `.specify/plans/add-oauth/workspace.md`.

### Propose with project assignment

During the propose phase, each proposed change is **assigned to a project**. The assignment uses three signals:

1. **Registry descriptions** -- the primary signal. Each change's description is matched against project descriptions.
2. **Baseline specs** -- capabilities already specified in a repo have strong affinity.
3. **Schema identity** -- a UI capability is unlikely to route to an `omnia` backend project.

For each slice, the agent shows the proposed assignment:

```
Proposed: add-token-management
  Description: OAuth2 token management and provider integration
  Project: api (inferred from: "authentication, token management")
  Accept / Edit / Reject?
```

You can override the assignment during the edit step.

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

For each change, the driver:

1. Reads the `project` field.
2. Routes to the correct repo in `.specify/workspace/<project>/`.
3. Uses the project's schema for define/build/merge.
4. Writes phase outcomes and transitions the plan entry.

Changes execute in dependency order. `add-token-management` (api) runs first, then `add-login-screens` (mobile) can start because its dependency is `done`.

## 6. Greenfield variant

For new platforms where repos do not exist yet, the registry describes the *intended* organisation:

```yaml
projects:
  - name: api
    url: .
    schema: omnia@v1
    description: >
      REST API and business logic.

  - name: mobile
    url: ../mobile
    remote: git@github.com:org/mobile.git
    schema: vectis@v1
    description: >
      iOS and Android mobile application.
```

When `/spec:execute` encounters a project with no `.specify/` directory, it runs `specify init --schema <url>` to bootstrap it before proceeding with define.

## What you learned

- `registry.yaml` declares the repos in your platform with domain descriptions.
- `/spec:plan` automatically syncs peers when the registry has multiple projects.
- Changes are assigned to projects based on description matching, baseline specs, and schema.
- `plan.yaml` entries carry a `project` field for routing.
- `/spec:execute` routes each change to the correct repo and schema.
- The same flow works for brownfield (existing repos) and greenfield (new repos).

## Next

[Legacy Migration at Scale](legacy-migration-at-scale.md) -- decompose a large monolith across multiple target repos using the analyze/extract split.
