# Transcript — `/spec:plan platform-modernisation`

This transcript pins the dialogue and the four shell-outs the
registry-proposal sub-step (RFC-9 §2B) triggers when an unresolved
assignment names a project that does not yet exist in
`registry.yaml`.

The transcript starts at the **end** of step 3(d) Assignment, after
the propose brief has written all three plan entries via
`specify plan add` and the assignment table has been rendered with
the third row marked unresolved (`?`).

```text
## Assignment

| # | Entry                     | Project          | Rationale                                                       |
|---|---------------------------|------------------|-----------------------------------------------------------------|
| 1 | ingest-pipeline-refactor  | legacy-monolith  | description overlap: ingest, queue, back-pressure               |
| 2 | operator-dashboard-alerts | command-centre   | baseline spec: user-alerts exists on command-centre             |
| 3 | alpha-gateway-extract     | ?                | ambiguous: no existing project owns the gateway layer           |

Resolve the assignment for `alpha-gateway-extract`. Pick one of:
  legacy-monolith
  command-centre
  <new project name>

> alpha-gateway

Project `alpha-gateway` does not exist in registry.yaml. Create it now? [y/N] y

Proposed defaults (edit any field, or accept all):
  --url    git@github.com:augentic/alpha-gateway.git   (inferred from existing prefix)
  --schema omnia@v1                                   (majority schema)

Accept the URL? [Y/n] y
Accept the schema? [Y/n] y

A description is required when the resulting registry has more than one
project. Enter a one-line description for `alpha-gateway`:

> Inbound traffic gateway carved out of the monolith's edge layer.

Running:
  $ specify registry add alpha-gateway \
        --url git@github.com:augentic/alpha-gateway.git \
        --schema omnia@v1 \
        --description "Inbound traffic gateway carved out of the monolith's edge layer."
  ok: appended `alpha-gateway` to registry.yaml (3 projects)

  $ specify workspace sync
  ok: materialised .specify/workspace/alpha-gateway/

  $ specify plan amend alpha-gateway-extract --project alpha-gateway
  ok: alpha-gateway-extract.project = alpha-gateway

Re-rendering assignment table:

| # | Entry                     | Project          | Rationale                                                       |
|---|---------------------------|------------------|-----------------------------------------------------------------|
| 1 | ingest-pipeline-refactor  | legacy-monolith  | description overlap: ingest, queue, back-pressure               |
| 2 | operator-dashboard-alerts | command-centre   | baseline spec: user-alerts exists on command-centre             |
| 3 | alpha-gateway-extract     | alpha-gateway    | greenfield carve-out — newly added registry entry (RFC-9 §2B)   |

All entries resolved. Continuing to step 4 (specify plan validate).

Running:
  $ specify plan validate
  PASS

Wrote .specify/plans/platform-modernisation/proposal.md.

Done. Next steps:
  - specify plan status
  - /spec:execute --loop
```

## Invariants pinned by this transcript

- **Decline default.** The confirm prompt defaults to `N`. The transcript shows the operator typing `y`; an `N` (or empty) response would surface a follow-up prompt asking the operator to either name an existing project or drop the entry via `specify plan transition <name> skipped`.
- **URL inference.** `git@github.com:augentic/<name>.git` is inferred from the longest common `<host>:<org>/` prefix across the existing entries (`legacy-monolith` and `command-centre`).
- **Schema default.** `omnia@v1` is the majority schema (both existing entries use it). On a tie the skill would prompt with the candidates instead.
- **Verb order.** `specify registry add` → `specify workspace sync` → `specify plan amend --project <new>`. Reversing any pair would surface a validator error or leave a clone slot unmaterialised.
- **`--description` is required.** Multi-project registries enforce `description-missing-multi-repo` (RFC-3b). The skill prompts unconditionally before calling `specify registry add`.
