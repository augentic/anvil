---
id: discovery
description: Read --from artefacts and/or analyse codebases; emit a neutral capability inventory.
generates: .specify/plans/<name>/discovery.md
---

Produce a neutral, schema-agnostic capability inventory for the
initiative. Discovery is read-only: it does NOT write to `plan.yaml`
and does NOT propose slices. Its only output is the inventory that
`propose.md` will decompose.

## Inputs

- `--from <path>...` — artefact files or directories authored by a
  human (briefs, RFCs, product docs, ADRs). Zero or more.
- `--against <path>` — an existing codebase to delta against. At
  most one. Interpreted as a local filesystem path.
- `--source <key>=<path-or-url>...` — named sources for migration
  or legacy analysis. `<path-or-url>` is either a local path or a
  git URL. Zero or more. The `<key>` is the identifier recorded on
  each plan entry's `sources` list in the next brief.

At least one of `--from`, `--against`, or `--source` must be
supplied.

## Process

1. **Analyse each `--source` and `--against` input.** For every
   non-`--from` input, invoke `/spec:extract` to produce a
   domain-level capability description:
   - For a git URL `--source`: clone via `/rt:git-cloner` into
     `legacy/<key>/` first, then run `/spec:extract legacy/<key>
     .specify/plans/<name>/extract/<key>/`.
   - For a local path `--source` or `--against`: run
     `/spec:extract <path> .specify/plans/<name>/extract/<key>/`
     directly (use `against` as the key for `--against`).
   - `/spec:extract` composes Omnia's `git-cloner` and `analyze`
     plugins; discovery does not invoke them directly.
   - The extract artefacts under `.specify/plans/<name>/extract/`
     are intermediate — the inventory below is the only
     human-facing output.
2. **Read each `--from` artefact.** Open every `--from` file (or
   every file under a `--from` directory). Parse any clearly
   delimited capability structure (e.g. headings named
   "Capability", "Service", "Bounded context"); otherwise treat
   each top-level heading as a capability candidate and record
   the accompanying prose verbatim.
3. **Merge into a single inventory.** Deduplicate capabilities
   that recur across sources (e.g. "user registration" in both a
   brief and a monolith extract). Record every source that
   mentions a capability rather than picking one.
4. **Write `.specify/plans/<name>/discovery.md`.** The output has
   a fixed shape (see "Output" below). Overwrite any existing
   file.

## Output

```markdown
# Discovery — <initiative-name>

## Capability inventory

### <capability name>

- **Source(s)**: <key>, <path>, <literal artefact path>, ...
- **Description**: <one or two sentences, source-neutral>
- **Ordering hints**: <e.g. "depends on user-accounts", "consumed
  by checkout"; omit if none>
- **Scope hints**: <e.g. "legacy monolith handler", "new greenfield
  service", "cross-cutting refactor"; omit if none>

<!-- repeat one subsection per capability -->

## Open questions

- <question requiring human input before proposal>
- <...>
```

## Idempotency

Running discovery twice on the same inputs MUST produce the same
`discovery.md`. Implications:

- Order capabilities stably (alphabetical within each source tier,
  then alphabetical overall).
- Do not include timestamps, run IDs, or working-directory paths.
- `/spec:extract` re-runs on unchanged sources must yield
  equivalent inventory text; if a re-extract surfaces new detail,
  it replaces the prior inventory entry wholesale.

## Example fragment

```markdown
# Discovery — platform-v2

## Capability inventory

### user-registration

- **Source(s)**: monolith (legacy/monolith), brief
  (briefs/platform-v2.md)
- **Description**: Self-service account creation with email +
  password; emits a verification email.
- **Ordering hints**: depended on by email-verification,
  shopping-cart.
- **Scope hints**: legacy monolith handler; greenfield target is
  a standalone Omnia crate.

### email-verification

- **Source(s)**: monolith (legacy/monolith)
- **Description**: Confirms the verification token produced by
  user-registration and flips the account to active.
- **Ordering hints**: depends on user-registration.

### shopping-cart

- **Source(s)**: orders (git@github.com:org/orders-service.git)
- **Description**: Line-item cart state for a session; no
  checkout or payment logic.
- **Ordering hints**: depends on user-registration; consumed by
  checkout-api.

## Open questions

- Is email-verification in scope for the first migration wave, or
  can we defer it behind a feature flag?
- Does orders-service own its own user model, or does it read
  through to the monolith?
```
