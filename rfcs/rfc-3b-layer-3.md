# RFC-3b: Federation at Execution Time (Layer 3)

> Status: Draft · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-2](archive/rfc-2-execution.md), [RFC-3](rfc-3a-monoliths.md)
> Out of scope for RFC-3. This document exists as a placeholder for the
> contract; the design is not yet ready to implement.

## Abstract

RFC-3 lands initiative *planning* across repos: a platform catalogue
(`registry.yaml`), an operator-authored brief (`initiative.md`), a
*sync peers* phase that materialises `.specify/workspace/<peer>/`, and
a single cross-repo `plan.yaml`. This RFC-3b covers what happens at
*execution* time once those peers exist: how specs in one repo refer
to capabilities in another, how provider/consumer contracts are
reconciled across the workspace, and how peer statuses roll up into
the initiating repo.

It is a follow-up to RFC-3 rather than a layer of it because the
planning flow (RFC-3 Layers 1–2 + Large-Monolith Decomposition) is
independently useful, and the execution-time surface is substantially
less well specified than the planning surface. Shipping RFC-3 without
this layer unblocks real multi-repo planning today; this document
catches up as experience accrues.

## Scope

Layer 3 is the smallest possible addition to RFC-2's per-repo
execution loop once the workspace exists:

- **Cross-repo spec references.** `@peer:capability` syntax in spec
  bodies. The CLI resolves against `.specify/workspace/<peer>/specs/`.
- **Contract reconciliation.** `specify federation validate` compares
  provider / consumer contracts declared across repos and flags
  mismatches across the workspace.
- **Peer status aggregation.** Read-only roll-up of peer change
  statuses into the initiating repo.

Layer 3 reads the same `.specify/workspace/` that RFC-3 Layer 2
materialises, so no new cloning, config, or peer discovery is
required.

## Open questions

The following are explicitly unresolved and blocking further
specification:

- **Resolution rules for `@peer:capability`.** What form does the
  reference take in a spec body? How does it resolve when the peer's
  baseline drifts between authoring and execution? What is the error
  mode when the referenced capability has been renamed or removed?
- **Contract schema.** What does a "provider contract" vs "consumer
  contract" look like on disk? Does it live in `specs/` alongside
  behavioural specs, or in a dedicated artefact? Who writes it —
  define? build? A new phase?
- **Reconciliation semantics.** What counts as a mismatch? Version
  drift? Missing capability? Type-shape drift? How does
  `specify federation validate` report?
- **Status aggregation.** Which peer statuses matter (change status,
  plan status, both)? How are they cached, and what invalidates the
  cache? Does the roll-up write a file, or is it computed on demand?
- **Workspace freshness under execution.** RFC-3 says
  `.specify/workspace/` is rebuilt by `specify initiative workspace
  sync` and otherwise read-only. Under execution, is the workspace
  re-synced automatically, only on explicit refresh, or never?

## Non-goals

- **Re-authoring planning-time behaviour.** Everything about how the
  plan is authored, how peers are synced at plan time, and how
  capabilities are discovered stays in RFC-3. RFC-3b is strictly an
  execution-time concern.
- **Introducing a new operator-facing skill.** Federation validation
  is a CLI verb (`specify federation validate`); execution continues
  to run through `/spec:execute`.
- **Write-access to peer clones during execution.** `.specify/workspace/`
  is read-only under RFC-3; RFC-3b preserves that.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md)
- [RFC-2: Execution](archive/rfc-2-execution.md)
- [RFC-3: Initiative Planning](rfc-3a-monoliths.md)
