# Proposal — platform-modernisation

## Slices

| # | Slice | Source(s) | Depends on | Decision | Plan entry |
|---|---|---|---|---|---|
| 1 | ingest-pipeline-refactor | monolith | — | accept | ingest-pipeline-refactor |
| 2 | operator-dashboard-alerts | monolith | ingest-pipeline-refactor | accept | operator-dashboard-alerts |
| 3 | alpha-gateway-extract | monolith | — | accept | alpha-gateway-extract |

## Assignment

| # | Entry | Project | Rationale |
|---|---|---|---|
| 1 | ingest-pipeline-refactor | legacy-monolith | Description overlap: ingest, queue, back-pressure on the monolith's existing surface. |
| 2 | operator-dashboard-alerts | command-centre | Baseline spec affinity: `user-alerts` already exists on `command-centre` and the slice extends it. |
| 3 | alpha-gateway-extract | **alpha-gateway** (new) | Greenfield carve-out — no existing project owns the gateway responsibility. Operator approved a new registry entry via the registry-proposal sub-step. |

## Registry amendments (RFC-9 §2B)

The registry-proposal sub-step (step 3(d).1) added one new project to `registry.yaml`:

| Name | URL | Schema | Description |
|---|---|---|---|
| alpha-gateway | git@github.com:augentic/alpha-gateway.git | omnia@v1 | Inbound traffic gateway carved out of the monolith's edge layer. |

Trigger: `alpha-gateway-extract` was unresolved during step 3(d) — no existing project (`legacy-monolith`, `command-centre`) was a clean owner for the gateway capability. The operator opted to create a new project rather than route the slice to one of the incumbents. Subsequent shell-outs ran in this exact order:

1. `specify registry add alpha-gateway --url git@github.com:augentic/alpha-gateway.git --capability omnia@v1 --description "Inbound traffic gateway carved out of the monolith's edge layer."`
2. `specify workspace sync`
3. `specify plan amend alpha-gateway-extract --project alpha-gateway`

## Notes

- Heuristics applied (Omnia, from `plugins/change/skills/plan/briefs/omnia/propose.md`): one slice per discovered capability; emit order follows `depends-on`.
- Slice 3 was originally surfaced with `confidence: medium` because no existing baseline spec covered the gateway layer; the operator confirmed the carve-out and approved the new registry entry.
- `specify plan validate` — no errors.
