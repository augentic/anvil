# Runtime capture extract

`/spec:refine` invokes this brief once per `slices[].sources[]` binding whose adapter is `captures`. Your job: for a single `(source-key, candidate-id)` pair, locate the matching `tests/data/replays/<handler>/` directory under `$SOURCE_DIR`, read every scenario capture, and emit one Evidence YAML document the CLI persists to `.specify/slices/<slice>/evidence/<source-key>.yaml`.

## Binding

The plan-level binding looks the same as `enumerate`'s:

```yaml
sources:
  runtime:
    adapter: captures
    path: ./captures/replays
```

The bound `path:` becomes `$SOURCE_DIR`. The capture layout is the one `/capture:wiretapper` writes — see [capture-format reference](../references/capture-format.md) for the per-file TestDef shape (`input`, `params`, `http_requests`, `output`; `setup` is test-harness-only).

## Reference shelf

Load on demand when extracting complex handler surfaces.

- [`../references/capture-format.md`](../references/capture-format.md) — on-disk wire format and behavioural vs non-evidence fields.
- [`../references/extraction-mapping.md`](../references/extraction-mapping.md) — capture JSON → Evidence claim field mapping.

## Inputs

- **`$SOURCE_DIR`** — read-only preopen of the bound capture root.
- **`<candidate-id>`** — the kebab-case id of the `## Candidate inventory` block this binding resolves to. It matches the `tests/data/replays/<candidate-id>/` directory name verbatim.
- **`<source-key>`** — the plan-level binding key under `plan.yaml.sources.<key>`.
- **`$SCRATCH_DIR`** — per-slice write-only scratch space; use only for unavoidable intermediate state.

`$PROJECT_DIR` is unreachable, host env is unreadable, the network is denied. Writes back into `$SOURCE_DIR` are denied.

## Claim grain

One `kind: example` claim per scenario file. A handler directory with 47 `<scenario>.json` files yields 47 example claims; synthesis fuses them later through the `requirement` / `criterion` claims contributed by sibling sources. The per-handler grain is the candidate (`enumerate`'s output); the per-scenario grain is the claim. This adapter does not collapse scenarios into a representative subset — every scenario the operator captured contributes one claim, and the 64 KiB inline cap (below) handles the bulk case.

## Output: Evidence YAML

Return one Evidence document matching `schemas/evidence.schema.json`. The CLI atomically writes it to `evidence/<source-key>.yaml`; you produce the body. Top-level field order is fixed (`source`, `adapter`, `authority`, `candidate`, `claims`):

```yaml
source: <source-key>
adapter: captures
authority: behaviour
candidate: <candidate-id>
claims:
  - kind: example
    claim-id: <kebab-id>
    path: tests/data/replays/<candidate-id>/<scenario>.json
    replay-digest: sha256:<hex>
    statement: "<single-line summary of what the scenario demonstrates>"
    input:
      method: <verb-or-channel>
      route: <route-or-topic>
      body: { ... }
    output:
      status: <http-status-or-equivalent>
      side-effects:
        - kind: message-pub
          topic: <topic>
          payload-shape: { ... }
```

`adapter` is the literal `captures`. `authority` is the literal `behaviour` for every Evidence document this adapter emits; per-kind overrides via `authority-overrides:` are rarely needed (runtime captures are behaviour by definition). `claim-id`, `path`, `replay-digest`, and `statement` are required on every `kind: example` claim; `input`, `output`, and any other observed shape are open per-kind body fields documented below.

## Claim fields

- **`kind: example`** — the single claim kind this adapter emits. Spec / criterion / decision claims belong to `documentation`; `excerpt` / `type` / `call` claims belong to code source adapters.
- **`claim-id`** (required) — stable kebab-case id derived from `<candidate-id>` plus the scenario filename stem. Example: scenario `tests/data/replays/user-registration/happy.json` → `claim-id: user-registration.happy`. Synthesis keys cross-source fusion off this id, so stability matters more than prettiness.
- **`path`** (required) — relative path under `$SOURCE_DIR`, no anchors. Always the capture JSON file itself; no `#L<n>` ranges (the whole file is the citation).
- **`replay-digest`** (required) — sha256 of the capture file's exact byte contents, prefixed `sha256:`. The CLI's cache fingerprint keys against this value; recomputing it on every run is cheap and lets downstream tools detect capture drift without re-reading the body.
- **`statement`** (required) — single-line summary of what the scenario demonstrates. Quote concrete request / response shape; do not paraphrase generalities.
- **`input`** / **`output`** / additional per-kind body fields — open. Mirror the TestDef shape the capture itself records (`input`, `params`, `http_requests`, `output.success` / `output.failure`). `side-effects[]` carries observed published messages, scheduled jobs, or outbound calls with `kind`, `topic`, and a payload shape (not the raw payload — see the inline cap below).

## 64 KiB inline cap

The adapter MUST NOT emit capture bodies larger than 64 KiB inline. Over-budget claims carry only the required fields (`kind`, `claim-id`, `path`, `replay-digest`, `statement`) and omit `input` / `output`:

```yaml
  - kind: example
    claim-id: bulk-import.10k-records
    path: tests/data/replays/bulk-import/10k-records.json
    replay-digest: sha256:9c1f...
    statement: "Bulk import of 10 000 records returns 202 with import id; body too large to inline."
```

The 64 KiB ceiling counts the serialised YAML body fields for a single claim, not the underlying JSON file. Sum every inlined field per claim; when the sum would exceed 64 KiB, drop `input` / `output` and rely on `replay-digest` + `path` for downstream replay. The limit lives in this brief, not in `evidence.schema.json`, so a fork can raise it without a schema change.

## Determinism

- Emit claims in scenario-filename alphabetical order. Stable order keeps synthesis golden runs reproducible.
- Compute `replay-digest` over the raw file bytes (no normalisation, no re-serialisation). Two adapters that hash the same file MUST produce the same digest.
- `claim-id` derives mechanically from `<candidate-id>` + the scenario stem (filename without `.json`, kebab-cased). Do not invent prettier ids; re-extraction must produce byte-identical claims.
- Quote observed request and response shapes verbatim from the capture. Light structural compression (omitting null fields, collapsing repeated array entries to one + count) is acceptable; semantic rewriting is not.

## Worked example

Bound candidate `user-registration` against the capture tree from the `enumerate` brief's worked example, source key `runtime`. Three scenarios; each fits inline under the 64 KiB cap.

Resulting Evidence YAML:

```yaml
source: runtime
adapter: captures
authority: behaviour
candidate: user-registration
claims:
  - kind: example
    claim-id: user-registration.duplicate-email
    path: tests/data/replays/user-registration/duplicate-email.json
    replay-digest: sha256:1a4b...
    statement: "POST /users with an email already in the store returns 409 with `{ error: duplicate-email }`; no message published."
    input:
      method: POST
      route: /users
      body: { email: alice@example.com, password-hash: "$argon2..." }
    output:
      status: 409
      body: { error: duplicate-email }
  - kind: example
    claim-id: user-registration.happy
    path: tests/data/replays/user-registration/happy.json
    replay-digest: sha256:7a2b...
    statement: "POST /users with a fresh email returns 201 and publishes `user.created` with the new user-id."
    input:
      method: POST
      route: /users
      body: { email: bob@example.com, password-hash: "$argon2..." }
    output:
      status: 201
      side-effects:
        - kind: message-pub
          topic: user.created
          payload-shape: { user-id: uuid, email: string }
  - kind: example
    claim-id: user-registration.invalid-password
    path: tests/data/replays/user-registration/invalid-password.json
    replay-digest: sha256:3c8e...
    statement: "POST /users with a password failing strength rules returns 400 with `{ error: weak-password }`."
    input:
      method: POST
      route: /users
      body: { email: carol@example.com, password-hash: "abc" }
    output:
      status: 400
      body: { error: weak-password }
```

Three scenarios, three claims, three digests. Synthesis fuses these with sibling sources' `requirement` and `criterion` claims to populate `spec.md`'s `Sources: [..., runtime]` lines.

## Path rules

Same skip-root and traversal rules as `enumerate`: relative paths only under `$SOURCE_DIR`, no `..`, no leading `/`, never above `tests/data/replays/`. A symlink inside `$SOURCE_DIR` pointing outside is denied at canonicalization; the host runner returns `source-extract-path-denied` and the slice stays `refining`.

## Anti-patterns

- **Inlining over-budget bodies.** Respect the 64 KiB inline cap. Over-budget claims fall back to `replay-digest` + `path`; downstream replay reads the bytes from disk.
- **Representative-scenario shortcuts.** Every captured scenario contributes one claim. Collapsing 47 scenarios into 3 "representative" examples loses the divergence signal that makes runtime authority useful.
- **Speculative claims.** Do not infer behaviour the captures do not exhibit. If no capture demonstrates duplicate-email handling, emit no claim for it — synthesis tags unknowns; you do not.
- **`INSTRUCTIONS.md` as evidence.** The per-handler `INSTRUCTIONS.md` is operator hint material for Omnia test generation ([`build/test.md`](../../../targets/omnia/briefs/build/test.md)); not behavioural evidence. Read it for surface-naming context if needed; do not turn its prose into claims.
- **Whole-file dumps in `statement`.** The `path:` + `replay-digest:` pair is the citation; `statement:` is a single-line summary. The body fields (`input` / `output`) carry observed structure; raw JSON paste in `statement:` is wrong.
- **Cross-source synthesis.** Do not fuse this candidate's claims with another source's Evidence — that is core synthesis's job in `/spec:refine`. Emit Evidence purely from `$SOURCE_DIR`.

## Failure modes

| Condition | Action |
| --- | --- |
| Candidate's `<handler>/` directory missing or empty under `$SOURCE_DIR` | Return `claims: []`. Synthesis surfaces `[unknown]` requirements. |
| Scenario JSON unparseable | Skip the scenario, continue with siblings. The slice surfaces partial Evidence; the operator decides whether to repair upstream or accept the gap. |
| Read denied outside `$SOURCE_DIR` / `$CAPABILITY_DIR` | Host runner returns `source-extract-path-denied`; slice stays `refining`. |
| `evidence.schema.json` validation fails on emit | CLI rejects the Evidence; slice stays `refining`. Re-emit with the missing required field (`claim-id`, `path`, `replay-digest`, `statement`) corrected. |

## References

- [RFC-27 §`extract` output](../../../../rfcs/archive/rfc-27-synthesis.md#extract-output)
- [RFC-27 §Runtime source adapter (D1)](../../../../rfcs/archive/rfc-27-synthesis.md#runtime-source-adapter-d1)
- [Capture format reference](../references/capture-format.md)
- [Extraction mapping reference](../references/extraction-mapping.md)
