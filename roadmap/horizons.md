# Horizon Deep Dive

_Exported on 14/04/2026 from Cursor_

---

## Horizon 1: The "Deferred" Validation Category

The most important architectural decision in the CLI is the three-way classification of validation results: `Pass`, `Fail`, `Deferred`. This is what makes the inversion-of-control model work in practice.

Deterministic frameworks (OpenSpec, SpecKit) can only do `Pass`/`Fail` because they have no agent to handle ambiguity. They either over-reject (blocking on rules they can't evaluate) or under-validate (skipping semantic rules entirely). Specify's model says: the CLI handles the structural checks, flags what it can't evaluate, and the agent applies judgment on the remainder. The agent's prompt surface for validation shrinks from "evaluate these 15 rules against this artifact" to "evaluate these 3 deferred rules that the CLI couldn't check."

This pattern generalises. Any time you're tempted to add a complex heuristic to the CLI, ask: "Is this better as a `Deferred` result that the agent evaluates?" The CLI should be conservative — it's better to defer a check to the agent than to implement a brittle heuristic that produces false positives.

### Classification Heuristic

For each validation rule string in `schema.yaml`, the CLI applies a pattern-matching heuristic to decide whether it can handle the rule deterministically:

| Rule pattern | Classification | Example |
|---|---|---|
| "Has a X section" | Structural — check heading exists | `Pass`/`Fail` |
| "Has a X section with at least one Y" | Structural — check heading + content | `Pass`/`Fail` |
| "Every requirement has at least one scenario" | Structural — parsed spec check | `Pass`/`Fail` |
| "Uses X format" (WHEN/THEN, checkbox, etc.) | Structural — regex check | `Pass`/`Fail` |
| "IDs use the REQ-XXX format" | Structural — regex check | `Pass`/`Fail` |
| "Uses SHALL/MUST language" | Semantic — requires NLP | `Deferred` |
| "Crate names are kebab-case" | Structural — regex check | `Pass`/`Fail` |

Rules that don't match any known pattern default to `Deferred`. This ensures the CLI never silently passes a rule it doesn't understand.

### What the Agent Receives

After running `specify validate`, the skill receives a structured report. Its responsibility is limited to:

1. Reporting `Fail` results to the user with suggested fixes.
2. Evaluating `Deferred` results using semantic understanding.
3. Deciding whether to proceed, fix, or ask for guidance.

The agent never has to count sections, verify ID patterns, or check dependency graphs. These are the operations most prone to LLM error and are now handled by the CLI.

---

## Horizon 2: Config as Schema, Not Prose

### The Problem

The current `defaults.rules` in `schema.yaml` contains multi-line prose strings:

```yaml
rules:
  proposal: |
    - Identify the proposal source. It will be either a git repository URL
      (for code analysis) or a manual proposal (feature work).
    - Use crate names. For modified crates, use the existing spec folder name
      from .specify/specs/. For new crates, choose a name that will be the
      crate name
```

These are instructions to the agent, stored in a configuration file that's also consumed by the CLI. The CLI has to parse YAML, extract a string, and ignore it (because it's prose the agent reads). This conflates two concerns.

### The Solution: Separate Structure from Guidance

A cleaner separation for config v2:

- **`schema.yaml`** contains *structural* configuration the CLI validates: blueprint DAGs, ID patterns, file patterns, validation flags.
- **`instructions/*.md`** contains *behavioural* guidance the agent follows: how to write proposals, how to structure specs, what to include in design.
- **`defaults.rules`** becomes **`defaults.guidance`** and each value is a *file path* to a guidance document, not inline prose:

```yaml
defaults:
  context:
    language: rust
    framework: omnia-sdk
  guidance:
    proposal: instructions/guidance/proposal.md
    specs: instructions/guidance/specs.md
```

The CLI resolves the path; the agent reads the file. No prose in YAML.

### Proposed `config.yaml` v2

```yaml
version: 2
schema: omnia@v1

project:
  name: my-service
  language: rust
  description: |
    A user authentication service built on Omnia SDK.

overrides:
  specs:
    format: given-when-then
  tasks:
    greenfield-chain: [guest-writer, crate-writer, test-writer, code-reviewer]
```

Key changes:

1. **`project` block** replaces the freeform `context` string with structured fields the CLI can use for validation and the agent can use for context. The `description` field remains freeform for agent consumption.
2. **`overrides` uses structured keys** instead of prose strings where possible. The string-based rules (like "Use WHEN/THEN format") become named options the CLI validates. Prose-only overrides stay as freeform strings under an `instructions` key.
3. **Schema resolution is a CLI concern**, not a skill concern. The `schema` field is just an identifier; `specify schema resolve` handles the rest.
4. **`.metadata.yaml` stays per-change** but gets validated by the CLI instead of the agent re-reading and verifying status values.

### Migration Path

The CLI supports both v1 and v2 config formats. `specify init` generates v2. A `specify migrate-config` subcommand upgrades v1 configs.

---

## Horizon 3: Multi-Repo Coordination

### The Problem

A feature like "add OAuth login" might require:

- Backend repo: new auth handler crate, updated API specs
- Frontend repo: new login screen, token storage
- Shared repo: updated API contract types

Each repo has its own `.specify/` directory with its own specs. The feature spans all three but there's no coordination point.

### Federated Specs Model

Instead of a separate "registry repo," extend the existing `config.yaml` to declare **peer repositories** and use the CLI to coordinate.

```yaml
# .specify/config.yaml in the backend repo
version: 2
schema: omnia@v1

project:
  name: auth-backend

federation:
  - name: auth-frontend
    repo: git@github.com:org/auth-frontend.git
    specs: .specify/specs
  - name: shared-types
    repo: git@github.com:org/shared-types.git
    specs: .specify/specs
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
### Requirement: OAuth Token Exchange
ID: REQ-004

The system SHALL exchange authorization codes for access tokens
using the endpoint defined in @shared-types:oauth/spec.md#REQ-002.
```

**4. `specify federation validate`** checks cross-repo references resolve and flags conflicts where the same API contract is specified differently in different repos.

### Why Not a Registry Repo

A separate registry repo creates a coordination bottleneck. Every change requires commits to the registry, and the registry becomes a merge conflict magnet. The federated model keeps each repo autonomous — the `federation` config is a declaration of *which* repos are related, and the CLI does lightweight reconciliation. If you later want a central dashboard or CI check, you can build it on top of the federation manifests without requiring a separate repo.

### The Exception: Cross-Organisation Coordination

If you're coordinating across *organisations* (not just repos), a registry repo makes more sense because you can't assume write access to peer repos. In that case, the registry holds the change manifests and the peer spec snapshots, and the CLI treats it as a read-only reference. But start with the federated model for the single-organisation case.

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

---

## On the `!`\`command\` Syntax in Skills

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
