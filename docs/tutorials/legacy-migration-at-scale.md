# Legacy Migration at Scale

Migrating a large legacy codebase is one of the most demanding uses of Specify. This tutorial shows how to decompose a monolith into a multi-repo, multi-change initiative using the analyze/extract split -- the strategy that makes this tractable.

**Prerequisites:** Familiarity with [multi-change initiatives](single-repo-initiative.md) and [cross-repo planning](cross-repo-initiative.md).

## The scaling challenge

A legacy monolith might have hundreds of thousands of lines of code. You cannot extract full specs from the entire codebase in one pass -- it would be too slow, too expensive, and the output would be overwhelming.

Specify solves this with a **two-skill split**:

| Skill | When | Depth | Scope | Cost |
|-------|------|-------|-------|------|
| `/spec:analyze` | Plan time | Shallow (capability summaries) | Entire source | Low |
| `/spec:extract` | Define time | Deep (full specs + design) | Per-change slice | Higher, but focused |

At plan time, analyze scans the whole monolith cheaply to build an inventory. At define time, extract runs deeply against only the files relevant to each change.

## 1. Set up the registry

Define the target repos where migrated capabilities will land:

```yaml
# .specify/registry.yaml
version: 1
projects:
  - name: auth-service
    url: ../auth-service
    remote: git@github.com:org/auth-service.git
    schema: omnia@v1
    description: >
      Authentication and authorization. Token management,
      OAuth providers, session handling, RBAC.

  - name: order-service
    url: ../order-service
    remote: git@github.com:org/order-service.git
    schema: omnia@v1
    description: >
      Order processing. Cart management, checkout flow,
      payment integration, order lifecycle.

  - name: notification-service
    url: ../notification-service
    remote: git@github.com:org/notification-service.git
    schema: omnia@v1
    description: >
      Notification dispatch. Email, SMS, push notifications,
      template management, delivery tracking.
```

## 2. Plan the migration

Point the plan skill at the monolith:

```text
/spec:plan modernise-platform --source monolith=/path/to/legacy-monolith
```

### Discovery phase

`/spec:analyze` scans the entire monolith at plan time. For each discovered capability, it emits a summary to `discovery.md`:

```markdown
### token-validation

Summary: Validates JWT tokens and checks expiry, signature, and claims.
Sources: src/auth/token.ts, src/auth/jwt.ts, src/middleware/auth.ts
Depends-on: [session-management]
Confidence: high
```

It also produces structural metadata at `.specify/plans/modernise-platform/analyze/monolith/metadata.json`:

```json
{
  "language": "TypeScript",
  "loc": 245000,
  "modules": 47
}
```

This is intentionally shallow. Analyze identifies *what capabilities exist* and *where they live*, not the full behavioral specification.

### Workspace sync

Because the registry declares multiple projects, the sync-peers phase clones the target repos and inventories them. For greenfield targets, this confirms they have no existing baselines.

### Propose phase

The propose phase matches discovered capabilities to projects using:

1. Capability descriptions from `discovery.md`.
2. Project descriptions from `registry.yaml`.
3. Dependency edges between capabilities.

Each proposed change is presented for review:

```
Proposed: extract-token-validation
  Description: Migrate token validation from the monolith
  Project: auth-service
  Sources: [monolith]
  Depends-on: []
  Accept / Edit / Reject?
```

## 3. Handle tangled code

Legacy monoliths often have tangled dependencies. A capability's source files may be scattered across multiple modules, or a single file may contain logic for multiple capabilities.

### Manifest scopes

For tangled codebases, you can provide a **manifest** -- a file listing exactly which source files a change should extract from:

```text
# migration-manifest.txt
src/auth/token.ts
src/auth/jwt.ts
src/middleware/auth.ts
src/shared/crypto.ts
```

During the propose phase, you can edit a change to include a manifest reference. At define time, `/spec:extract` uses the manifest to scope its deep analysis.

### Overlapping changes

When multiple changes touch the same source files, `specify change overlap` detects the overlap and reports it. You can then:

- Merge the changes into one.
- Sequence them with `depends-on` so one extracts first.
- Accept the overlap if the capabilities are truly independent.

## 4. Execute the migration

```text
/spec:execute --loop
```

For each change in dependency order:

1. **Define** -- `/spec:define` invokes `/spec:extract` against the monolith source files relevant to this change. This is the **deep extraction** -- full behavioral specs and design.
2. **Build** -- Specialist skills generate the new implementation in the target repo.
3. **Merge** -- Specs merge into the target repo's baseline.

Each merged change adds to the target repo's baseline. Subsequent changes see the accumulated baseline and produce deltas against it.

## 5. Mix extraction and greenfield changes

A migration plan often includes both:

- **Extraction changes** -- migrating existing capabilities from the monolith (have `sources`).
- **Greenfield changes** -- adding new capabilities that do not exist in the legacy code (no `sources`).

Both types coexist in a single plan. The define phase handles them differently:

- Extraction changes invoke `/spec:extract` to derive specs from source.
- Greenfield changes generate specs from the description alone.

```yaml
changes:
  - name: extract-auth
    description: "Migrate authentication from monolith"
    sources: [monolith]
    project: auth-service
    status: pending

  - name: add-oauth
    description: "Add OAuth2 provider support (not in legacy)"
    depends-on: [extract-auth]
    project: auth-service
    status: pending
```

## 6. Verify across repos

After the migration completes, verify each target repo:

```text
# In each target repo
/spec:verify
```

This confirms that the generated code matches the extracted and newly authored specifications.

## The migration workflow

```text
# One-time setup
Create registry.yaml with target repos
/spec:plan modernise-platform --source monolith=/path/to/legacy

# Review
specify initiative status

# Execute
/spec:execute --loop

# Verify
(per repo) /spec:verify
```

## What you learned

- The **analyze/extract split** makes large migrations tractable: cheap scanning at plan time, deep extraction at define time.
- Discovery produces capability summaries with source-file hints and dependency edges.
- The propose phase matches capabilities to target projects.
- Tangled code is handled with manifest scopes and overlap detection.
- Extraction and greenfield changes coexist in a single plan.
- Baseline accumulation in target repos gives subsequent changes context.
