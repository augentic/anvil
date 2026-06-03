# Shared spec runtime references

Symlinks point at canonical files under `plugins/spec/references/` (and `review-team-protocol.md` under `docs/reference/`). `specrun init` vendors dereferenced copies into each cached target adapter at `references/spec-runtime/` so briefs can link with `../references/spec-runtime/...` inside the adapter tree only.

| Symlink | Canonical |
| --- | --- |
| `guardrails.md` | `plugins/spec/references/guardrails.md` |
| `specialist-usage.md` | `plugins/spec/references/specialist-usage.md` |
| `reconciliation.md` | `plugins/spec/references/reconciliation.md` |
| `components.md` | `plugins/spec/references/components.md` |
| `standards-layer-snippet.md` | `plugins/spec/references/standards-layer-snippet.md` |
| `artifact-validation-checklist.md` | `plugins/spec/references/artifact-validation-checklist.md` |
| `cli/plan-propose.md` | `plugins/spec/references/cli/plan-propose.md` (symlink uses five `../` segments — nested under `runtime/cli/`) |
| `synthesis/authority.md` | `plugins/spec/references/synthesis/authority.md` (symlink uses five `../` segments — nested under `runtime/synthesis/`) |
| `review-team-protocol.md` | `docs/reference/review-team-protocol.md` |
| `phase-outcome-contract.md` | `plugins/spec/references/phase-outcome-contract.md` |

Do not add agent-critical prose only under `docs/` — extend the plugin canonical file, then refresh symlinks here.

## Edit the canonical source only

The per-adapter `references/spec-runtime/` trees (~120 files across `adapters/{sources,targets}/*/`) are **generated artifacts**, not editable sources. They are materialised by [`scripts/sync-adapter-spec-runtime.sh`](../../../../scripts/sync-adapter-spec-runtime.sh) from the canonical files listed above (plus `synthesis/{authority,tags,provenance,claim-reconciliation}.md`, `cli/plan-propose.md`, `stop-conditions.md`, and `plan-lock.md`). Edit the canonical file — never hand-edit a materialised `references/spec-runtime/` copy; the next sync will overwrite it.

After editing any canonical file, run `bash ./scripts/sync-adapter-spec-runtime.sh` (or `make lint` / `make ci`, which both run the `sync-spec-runtime` target first) so the materialised copies match, then commit the regenerated trees in the same change. There is currently no CI diff gate that fails when the materialised trees drift from canonical — a `sync-spec-runtime`-then-`git diff --exit-code` gate is a tracked follow-up; until it lands, the canonical-only discipline above is enforced by convention.

Per-target `references/agent-teams.md` symlinks to `review-team-protocol.md` here so review briefs keep a stable relative link; `specrun init` vendors the dereferenced bytes into `references/spec-runtime/review-team-protocol.md`.
