# Runtime capture extract

`/spec:refine` invokes this brief once per `slices[].sources[]` binding whose adapter is `captures`. Your job: for a single `(source, lead)` pair, locate the matching `tests/data/replays/<handler>/` directory under `$SOURCE_DIR`, read every scenario capture, and emit one Evidence YAML document the CLI persists to `.specify/slices/<slice>/evidence/<source>.yaml`.

## Binding

The plan-level binding looks the same as `survey`'s:

```yaml
sources:
  runtime:
    adapter: captures
    path: ./captures/replays
```

The bound `path:` becomes `$SOURCE_DIR`. The capture layout is the one `/capture:wiretapper` writes — see [capture-format reference](../references/capture-format.md) for the per-file TestDef shape (`input`, `params`, `http_requests`, `output`; `setup` is test-harness-only).

## Reference shelf

Load both references — they own everything the brief does not spell out.

- [`../references/capture-format.md`](../references/capture-format.md) — on-disk wire format and behavioural vs non-evidence fields.
- [`../references/extraction-mapping.md`](../references/extraction-mapping.md) — Evidence YAML shape, claim-field detail, capture JSON → claim field mapping, 64 KiB inline cap, determinism rules, worked example, path rules, anti-patterns, and failure modes.

## Inputs

- **`$SOURCE_DIR`** — read-only preopen of the bound capture root.
- **`<lead>`** — the kebab-case id of the `## Lead inventory` block this binding resolves to. It matches the `tests/data/replays/<lead>/` directory name verbatim.
- **`<source>`** — the plan-level binding key under `plan.yaml.sources.<key>`.
- **`$SCRATCH_DIR`** — per-slice write-only scratch space; use only for unavoidable intermediate state.

`$PROJECT_DIR` is unreachable, host env is unreadable, the network is denied. Writes back into `$SOURCE_DIR` are denied.

## Claim grain

One `kind: example` claim per scenario file. A handler directory with 47 `<scenario>.json` files yields 47 example claims; synthesis reconciles them later through the `requirement` / `criterion` claims contributed by sibling sources (see [From sources to slices](../references/spec-runtime/reconciliation.md#slice-time-evidence-becomes-a-spec)). The per-handler grain is the lead (`survey`'s output); the per-scenario grain is the claim. This adapter does not collapse scenarios into a representative subset — every scenario the operator captured contributes one claim, and the 64 KiB inline cap (see references) handles the bulk case.

## Output skeleton

Emit Evidence YAML per [`extraction-mapping.md`](../references/extraction-mapping.md). Minimal shape:

```yaml
authority: behaviour
lead: <lead>
claims:
  - kind: example
    id: <lead>.<scenario-stem>
    path: tests/data/replays/<lead>/<scenario>.json
    replay-digest: sha256:<hex>
    statement: "<single-line summary>"
    input: { ... }
    output: { ... }
```

The CLI persists this at `.specify/slices/<slice>/evidence/<source>.yaml`, deriving the `(slice, source)` identity from the path and the adapter from `plan.yaml.sources.<source>.adapter`; neither is written in-document.

See the references for the full field table, the worked three-scenario example, determinism rules, the 64 KiB inline cap, path rules, anti-patterns, and failure modes.
