# RFC-81: Cloud Alert Remediation Platform

> **Status: Draft / exploratory.** Outside Emery’s core workflow loop. Depends: none. Owns: ingest → correlate → propose → human-approve for security and infrastructure alerts against Emery-owned surfaces.

## Abstract

A central cloud platform that listens for security advisories and infrastructure monitoring alerts, examines repositories and packages owned by Emery, and proposes fixes (draft PRs / tickets). It is an incident pipeline, not an Emery change workflow: use `plan → refine → build → merge` only when a fix is large enough to deserve a real change. Auto-propose; never auto-merge in v0.

## Motivation

Security and infra signals arrive continuously (advisories, Dependabot, monitoring webhooks). Today correlation and remediation are manual. A thin platform can:

- normalize and dedupe alerts;
- map them deterministically onto owned repos/packages;
- open a human-reviewed remediation proposal;
- leave lifecycle authority with existing CI gates and operators.

Emery’s CLI remains authoritative for product-change workflow state. This platform feeds engineering tools (`cargo audit` / deny / vet, Dependabot, CI); it does not reimplement slice or plan transitions.

## Pipeline

```text
Alerts (RSS / webhooks / polls)
  → normalize + dedupe
  → map to owned repos/packages
  → open a ticket / draft PR (propose only)
  → human review
```

## Week-1 MVP

### 1. Define “owned by Emery” as data

A small registry (YAML/JSON) of:

- repos (`augentic/emery`, `augentic/emery-adapters`, …)
- published packages / crate names / GHCR adapter images
- owners / on-call
- severity policy (e.g. critical/high auto-open; medium weekly digest)

### 2. Ingest the highest-signal feeds only

Prefer APIs over generic RSS:

- GitHub Advisories / Dependabot / code scanning alerts for owned repos
- `cargo audit` / OSV for Rust crates
- infra: Prometheus/Alertmanager, Datadog, CloudWatch → webhooks

RSS is fine as a secondary source (NVD, vendor blogs), but treat it as noisy and untrusted until correlated.

Useful advisory source: [GitHub REST API — global advisories](https://docs.github.com/en/rest/security-advisories/global-advisories).

### 3. Correlation before AI

Deterministic match first:

- advisory package ∈ lockfile / `Cargo.toml` / published adapter train
- alert labels ∈ known services/repos

Only then spend an agent turn. Most platform value is here, not in the LLM.

### 4. Propose-fix runner

For each correlated incident:

- clone/target the affected repo
- give the agent the alert + affected paths + constraints (minimal patch, don’t touch release process, run local CI gates when affordable)
- output: branch + draft PR + short risk note

Candidate runtimes:

- Cursor cloud agents via the [Cursor SDK](https://cursor.com/docs/sdk/typescript)
- A GitHub Action + `cursor-agent`

Decide early: cloud SDK vs CI-bound local agent.

### 5. Operator queue

Slack / Linear / GitHub Issues with severity, matched package/repo, proposed PR link, and actions: ignore / fix / escalate.

## Suggested architecture (v0)

| Piece | Choice for v0 |
| --- | --- |
| Trigger | cron (advisories) + webhooks (infra) |
| Store | Postgres or SQLite + object storage for alert payloads |
| Queue | one worker queue (Cloud Run / Fly / ECS) |
| Identity | GitHub App with least privilege (contents: write on fix branches, PRs) |
| Agent | one-shot “propose PR” job; no long-lived chat |
| Safety | allowlisted repos, max files touched, no secrets in prompts, draft PRs only |

## Relationship to Emery

- **Emery’s own codebases:** call normal engineering tools (`cargo deny` / audit, Dependabot, CI). The Emery CLI is not the security orchestrator.
- **Consumer projects managed with Emery:** route into `/emery:plan` only when the fix is a real product/spec change. Bumping a crate or patching a vuln is usually a normal PR, not a slice.
- **Existing gates:** Emery already treats supply-chain as CI (`cargo-vet`, `cargo-deny`, advisory audit). This platform should *feed* those, not replace them.

## First milestone

Ship this and stop:

> When a high/critical advisory hits a crate depended on in `emery` or `emery-adapters`, open a draft PR that bumps/patches it, with a comment linking the advisory.

That validates ingest → ownership map → propose. Add infra alerts and RSS only after that path is reliable.

## Open decisions

1. Scope: only Augentic-owned repos, or also customer Emery projects?
2. Runtime for the fixer: Cursor cloud agents, CI runners, or a private VM fleet?
3. Auto-open PRs vs tickets-only for the first month?
4. Source of truth for ownership: GitHub CODEOWNERS, a central registry, or both?

## Next design steps

When the decisions above are settled, flesh out:

- service layout (API routes, worker jobs)
- normalized alert schema
- ownership registry shape
- minimal “propose fix” prompt/contract
- PR labeling / draft conventions and human approval gates
