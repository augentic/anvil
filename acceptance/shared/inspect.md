# Inspecting run state

Read-only ways to see what an acceptance run produced. Prefer the CLI's own render verbs over hunting raw files: the rendered views are the canonical surface, they stay stable while on-disk artifact locations are still settling, and they read state the same way the skills and CI do.

Every run lives at the pinned sandbox `acceptance/.sandbox/<scenario>/` ([`setup.md`](setup.md)). Run these from that directory (or the routed project / workspace root within it). All are read-only — none drive a `/spec:*` command or mutate state.

## The event timeline

The single best "what happened, in order" view is the journal — a newline-delimited JSON log of the closed event taxonomy:

```bash
cat .specify/journal.jsonl                              # raw events
jq -c '{ts, kind}' .specify/journal.jsonl              # compact timeline (if jq is installed)
```

## Plan and change

```bash
specify plan validate --format json                    # structural + consistency findings
cat plan.yaml                                          # lifecycle, slices[], per-entry status, sources
```

## Slice artifacts

For a slice under `.specify/slices/<slice>/`:

```bash
specify slice model show <slice>                       # concise text view of model.yaml (add --format json for the verbatim model)
specify slice provenance <slice>                       # audit-only provenance, projected on demand (no provenance.yaml on disk)
specify slice validate <slice>                         # adapter validation findings
```

The synthesised Markdown artifacts (`proposal.md`, `specs/<slice>/spec.md`, `design.md`, `tasks.md`) sit alongside `model.yaml` in the slice directory and can be read directly.

## Workspace registry and topology

For registry-only workspace scenarios:

```bash
specify registry validate                              # registry.yaml shape
cat registry.yaml                                      # registered projects + routing descriptions
cat topology.lock                                      # materialised slot topology (after workspace sync)
```

## One-shot snapshot

To capture the whole picture at once — directory tree plus the key artifact bodies — for a run record, use the snapshot helper:

```bash
scripts/snapshot.sh "$SANDBOX"
```

It is read-only and prints text suitable for the **Artefact snapshot** section of [`run-summary-template.md`](run-summary-template.md).
