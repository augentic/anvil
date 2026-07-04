# Archived RFCs

RFCs land here when they are implemented (possibly in a changed shape) or superseded. They are kept verbatim apart from the status banner, which records the disposition. The current plan is [RFC-61](../rfc-61-omnia-migration.md), which replaced the S1–S4 staging these RFCs formed once the Omnia refactoring delivered the runtime in a different shape than they assumed.

| RFC | Disposition |
| --- | ----------- |
| [RFC-51 — Adapter WIT](rfc-51-adapter-wit.md) | Partially implemented (`wit/specify.wit` authored); the contract revision and bindings moved to RFC-61 |
| [RFC-52 — Effect Map](rfc-52-effect.md) | Superseded — the effect vocabulary landed as Omnia's implemented WASI host crates |
| [RFC-53 — `wasi-model` Host Core](rfc-53-wasi-model.md) | Implemented in a changed shape — Omnia's `omnia:model@0.1.0` `create`, gate validation, replay backend |
| [RFC-54 — Vertical Adapter Operation Proof](rfc-54-orchestration.md) | Superseded by RFC-61 Step 2 (the contracts adapter guest) |
| [RFC-56 — The Runtime Move](rfc-56-runtime-move.md) | Superseded — Omnia implemented the registry, instance-per-call, and link dispatch as a library with per-deployment `runtime!` binaries |
| [RFC-57 — Workflow and Development as Guests](rfc-57-specify-guests.md) | Superseded by RFC-61 Step 4 (the workflow guest) |
| [RFC-58 — Model Backends](rfc-58-model-backends.md) | Largely implemented — cursor, genai, and replay backends in `augentic/backends`; router / SLM not pursued |
| [RFC-59 — Model Tool Loop](rfc-59-model-tool-loop.md) | Implemented in a changed shape — Omnia's `ToolHost` plus the cursor backend's agent-side loop |
| [Implementation Sequence Plan](rfc-sequence.md) | Superseded by RFC-61's migration steps |

Still-pending relatives moved to [future/](../future/): [RFC-55 — Working-Tree Materialization](../future/rfc-55-working-tree.md) (deferred until a multi-node deployment exists) and [RFC-60 — Verify Profiles](../future/rfc-60-verify-profiles.md) (deferred while `verify` stays stubbed).
