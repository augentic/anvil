# RFC-3: Multi-Repo Coordination

> Status: Draft · Depends: [RFC-1](rfc-1-cli.md)

## Abstract

Extend `config.yaml` with a federation model — peer repositories declared in config, cross-repo spec references resolved by the CLI, and coordinated validation that catches contract mismatches across repo boundaries.

## Motivation

The `.specify/` directory is project-local. There is no concept of a spec reference that spans repositories, and conflict detection only works within a single workspace. A feature like "add OAuth login" that touches backend, frontend, and shared-types repos has no coordination point.

## Detailed Design

### Federated Specs Model

Instead of a separate "registry repo," extend the existing `config.yaml` to declare **peer repositories** and use the CLI to coordinate.

```yaml
# .specify/project.yaml
name: traffic
domain: |
  Traffic-related services including roadworks management,
  lane closures, and traffic flow analysis.
schema: omnia@v1

rules:
  proposal: rules/proposal.md
```

The multi-repo registry is a separate file that lives in a dedicated platform repo.

```yaml
# .specify/registry.yaml (lives in any repo, or a dedicated platform repo)
name: realtime
version: 1

projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    schema: omnia@v1

  - name: command-centre
    url: git@github.com:org/command-centre.git
    schema: omnia@v1
```

### How It Works

**1. `specify federation sync`** clones or fetches peer repo spec directories into `.specify/.peers/<name>/specs/` (read-only). This gives local skills visibility into cross-repo specs without modifying the peer repo.

**2. Cross-repo change coordination** uses a lightweight "change manifest" that lives outside any single repo:

```yaml
# auth-oauth-feature.yaml (could live in any repo or a shared location)
feature: add-oauth-login
changes:
  - repo: auth-backend
    change: add-oauth-handler
    status: building
  - repo: auth-frontend
    change: add-oauth-screen
    status: defined
  - repo: shared-types
    change: add-oauth-types
    status: complete
```

**3. Spec references across repos** use a `@repo:capability` syntax:

```markdown
## Requirement: OAuth Token Exchange
ID: REQ-004

The system SHALL exchange authorization codes for access tokens
using the endpoint defined in @shared-types:oauth/spec.md#REQ-002.
```

**4. `specify federation validate`** checks cross-repo references resolve and flags conflicts where the same API contract is specified differently in different repos.

### Phase A → B → C

**Phase A: Peer awareness.** Each repo's `config.yaml` declares its peers. `specify federation sync` pulls peer specs locally. Cross-repo spec references use `@peer:capability` syntax. This is enough for a small team working across 2-3 repos.

**Phase B: Change coordination.** When a feature spans repos, you create a "feature manifest" — a YAML file that lives in whichever repo initiates the feature. The `contracts` section declares cross-repo dependencies explicitly:

```yaml
feature: add-oauth-login
repos:
  - name: auth-backend
    change: add-oauth-handler
    ref: feature/add-oauth
  - name: auth-frontend
    change: add-oauth-screen
    ref: feature/add-oauth
contracts:
  - type: api
    provider: auth-backend
    consumer: auth-frontend
    spec: "@auth-backend:oauth/spec.md#REQ-002"
```

So `specify federation validate` can check that the API contract specified in the backend matches what the frontend expects.

**Phase C: Central dashboard (optional).** If you reach the point of coordinating across many repos or teams, you extract the federation manifests into a dedicated repo that aggregates them. But the manifests themselves are still authored in the feature repos — the central repo is a read-only aggregator, not a write-time dependency.

## Alternatives Considered

### Registry Repo

A separate registry repo creates a coordination bottleneck. Every change requires commits to the registry, and the registry becomes a merge conflict magnet. The federated model keeps each repo autonomous — the `federation` config is a declaration of *which* repos are related, and the CLI does lightweight reconciliation. If you later want a central dashboard or CI check, you can build it on top of the federation manifests without requiring a separate repo.

### Cross-Organisation Coordination

If you're coordinating across *organisations* (not just repos), a registry repo makes more sense because you can't assume write access to peer repos. In that case, the registry holds the change manifests and the peer spec snapshots, and the CLI treats it as a read-only reference. But start with the federated model for the single-organisation case.

## Appendix: On the `!`\`command\` Syntax in Skills

The `specify` CLI gives a clean abstraction boundary for deterministic execution in skills. Instead of skills containing scattered shell commands like:

```markdown
```bash
cd $CRATE_PATH && cargo test 2>&1 | tee /tmp/${CHANGE_ID}-baseline.txt
```
```

They can use:

```markdown
!`specify task capture-baseline --change "$CHANGE_DIR" --crate "$CRATE_NAME"`
```

This has two advantages over raw shell:

1. The CLI can handle platform differences (path separators, temp directories, shell quoting).
2. The CLI can return structured output that the skill parses, rather than raw stdout the agent must interpret.

However, not everything should go through the CLI. `cargo test`, `cargo fmt`, and `cargo clippy` are external tools that the agent should invoke directly — the CLI shouldn't wrap every possible build tool.

The principle: **the CLI owns Specify operations; external tool invocation stays with the agent.**

A good litmus test: "Would this command need to understand `.specify/` directory structure or spec format?" If yes, it belongs in the CLI. If no (like running `cargo test`), it stays as a direct shell command in the skill.

## References

- [RFC-1: `specify` CLI](rfc-1-cli.md) — prerequisite; federation subcommands extend the CLI
- [RFC-2: Feature Manifests](rfc-2-manifests.md) — provides manifest-level coordination for multi-repo initiatives; federation provides the resolution layer
