# Capture → Evidence extraction mapping

Maps runtime capture JSON fields to `kind: example` claim fields emitted by the `captures` extract brief. The brief at [`../briefs/extract.md`](../briefs/extract.md) keeps the binding, inputs, and claim grain; this reference owns the procedural detail — wire format, field mapping, caps, determinism, worked example, anti-patterns, failure modes.

## Output: Evidence YAML

Return one Evidence document matching `schemas/evidence.schema.json`. The CLI atomically writes it to `evidence/<source>.yaml`; the brief produces the body. Top-level field order is fixed (`authority`, `lead`, `claims`):

```yaml
authority: behaviour
lead: <lead>
claims:
  - kind: example
    id: <kebab-id>
    path: tests/data/replays/<lead>/<scenario>.json
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

The document's `(slice, source)` identity is path-borne (the CLI persists it at `.specify/slices/<slice>/evidence/<source>.yaml`) and the adapter resolves from `plan.yaml.sources.<source>.adapter`, so neither is written in-document. `authority` is the literal `behaviour` for every Evidence document this adapter emits; per-kind overrides via `authority-overrides:` are rarely needed (runtime captures are behaviour by definition). `id`, `path`, `replay-digest`, and `statement` are required on every `kind: example` claim; `input`, `output`, and any other observed shape are open per-kind body fields documented below.

## Claim fields

- **`kind: example`** — the single claim kind this adapter emits. Spec / criterion / decision claims belong to `documentation`; `excerpt` / `type` / `call` claims belong to code source adapters.
- **`id`** (required) — stable kebab-case id derived from `<lead>` plus the scenario filename stem. Example: scenario `tests/data/replays/user-registration/happy.json` → `id: user-registration.happy`. Synthesis keys cross-source reconciliation off this id, so stability matters more than prettiness.
- **`path`** (required) — relative path under `$SOURCE_DIR`, no anchors. Always the capture JSON file itself; no `#L<n>` ranges (the whole file is the citation).
- **`replay-digest`** (required) — sha256 of the capture file's exact byte contents, prefixed `sha256:`. A content anchor: recomputing it on every run is cheap and lets downstream tools detect capture drift without re-reading the body.
- **`statement`** (required) — single-line summary of what the scenario demonstrates. Quote concrete request / response shape; do not paraphrase generalities.
- **`input`** / **`output`** / additional per-kind body fields — open. Mirror the TestDef shape the capture itself records (`input`, `params`, `http_requests`, `output.success` / `output.failure`). `side-effects[]` carries observed published messages, scheduled jobs, or outbound calls with `kind`, `topic`, and a payload shape (not the raw payload — see the inline cap below).

## Field mapping

| Capture source | Evidence claim field | Rule |
|---|---|---|
| File path under `$SOURCE_DIR` | `path` | Relative path; no `#L` anchors — the whole file is the citation |
| Raw file bytes | `replay-digest` | `sha256:` prefix over exact bytes (no re-serialisation) |
| `<handler>/<stem>.json` | `id` | `<lead>.<stem>` kebab-case (mechanical derivation) |
| `input` + inferred HTTP surface | `input.method`, `input.route`, `input.body` | Quote observed shapes verbatim from capture |
| `params` | fold into `input` or omit | Include when they affect observed behaviour |
| `http_requests` | `input` outbound context or omit | Structural summary only when relevant to the scenario |
| `output.success` / `output.failure` | `output.status`, `output.body` | HTTP status or channel equivalent |
| Published messages in `output` | `output.side-effects[]` | `kind`, `topic`, payload **shape** — not raw bulk payloads |
| Scenario observation | `statement` | Single-line summary; not a JSON dump |
| Serialised claim body > 64 KiB | omit `input` / `output` | Required fields + digest + path only |

## 64 KiB inline cap

The adapter MUST NOT emit capture bodies larger than 64 KiB inline. Over-budget claims carry only the required fields (`kind`, `id`, `path`, `replay-digest`, `statement`) and omit `input` / `output`:

```yaml
  - kind: example
    id: bulk-import.10k-records
    path: tests/data/replays/bulk-import/10k-records.json
    replay-digest: sha256:9c1f...
    statement: "Bulk import of 10 000 records returns 202 with import id; body too large to inline."
```

The 64 KiB ceiling counts the serialised YAML body fields for a single claim, not the underlying JSON file. Sum every inlined field per claim; when the sum would exceed 64 KiB, drop `input` / `output` and rely on `replay-digest` + `path` for downstream replay. The limit lives in this reference, not in `evidence.schema.json`, so a fork can raise it without a schema change.

## Determinism

- Emit claims in scenario-filename alphabetical order. Stable order keeps synthesis golden runs reproducible.
- Compute `replay-digest` over the raw file bytes (no normalisation, no re-serialisation). Two adapters that hash the same file MUST produce the same digest.
- `id` derives mechanically from `<lead>` + the scenario stem (filename without `.json`, kebab-cased). Do not invent prettier ids; re-extraction must produce byte-identical claims.
- Quote observed request and response shapes verbatim from the capture. Light structural compression (omitting null fields, collapsing repeated array entries to one + count) is acceptable; semantic rewriting is not.

## Path rules

Same skip-root and traversal rules as `survey`: relative paths only under `$SOURCE_DIR`, no `..`, no leading `/`, never above `tests/data/replays/`. A symlink inside `$SOURCE_DIR` pointing outside is denied at canonicalization; the host runner returns `source-extract-path-denied` and the slice stays `refining`.

## Worked example

Bound lead `user-registration` against the capture tree from the `survey` brief's worked example, source key `runtime`. Three scenarios; each fits inline under the 64 KiB cap.

Resulting Evidence YAML:

```yaml
authority: behaviour
lead: user-registration
claims:
  - kind: example
    id: user-registration.duplicate-email
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
    id: user-registration.happy
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
    id: user-registration.invalid-password
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

Three scenarios, three claims, three digests. Synthesis reconciles these with sibling sources' `requirement` and `criterion` claims to populate `spec.md`'s `Sources: [..., runtime]` lines.

## Anti-patterns

- **Inlining over-budget bodies.** Respect the 64 KiB inline cap. Over-budget claims fall back to `replay-digest` + `path`; downstream replay reads the bytes from disk.
- **Representative-scenario shortcuts.** Every captured scenario contributes one claim. Collapsing 47 scenarios into 3 "representative" examples loses the divergence signal that makes runtime authority useful.
- **Speculative claims.** Do not infer behaviour the captures do not exhibit. If no capture demonstrates duplicate-email handling, emit no claim for it — synthesis tags unknowns; you do not.
- **`INSTRUCTIONS.md` as evidence.** The per-handler `INSTRUCTIONS.md` is operator hint material for Omnia test generation ([`build/test.md`](../../../targets/omnia/briefs/build/test.md)); not behavioural evidence. Read it for surface-naming context if needed; do not turn its prose into claims.
- **Whole-file dumps in `statement`.** The `path:` + `replay-digest:` pair is the citation; `statement:` is a single-line summary. The body fields (`input` / `output`) carry observed structure; raw JSON paste in `statement:` is wrong.
- **Cross-source synthesis.** Do not reconcile this lead's claims with another source's Evidence — that is core synthesis's job in `/spec:refine`. Emit Evidence purely from `$SOURCE_DIR`.

## Failure modes

| Condition | Action |
| --- | --- |
| Lead's `<handler>/` directory missing or empty under `$SOURCE_DIR` | Return `claims: []`. Synthesis surfaces `[unknown]` requirements. |
| Scenario JSON unparseable | Skip the scenario, continue with siblings. The slice surfaces partial Evidence; the operator decides whether to repair upstream or accept the gap. |
| Read denied outside `$SOURCE_DIR` / `$CAPABILITY_DIR` | Host runner returns `source-extract-path-denied`; slice stays `refining`. |
| `evidence.schema.json` validation fails on emit | CLI rejects the Evidence; slice stays `refining`. Re-emit with the missing required field (`id`, `path`, `replay-digest`, `statement`) corrected. |

## See also

- [`capture-format.md`](capture-format.md) — on-disk wire format
- [`../briefs/extract.md`](../briefs/extract.md) — binding, inputs, claim grain
- [workflow §`extract` output](https://github.com/augentic/specify/blob/main/engine/docs/standards/workflow.md#extract-output)
- [workflow §Runtime source adapter (D1)](https://github.com/augentic/specify/blob/main/engine/docs/standards/workflow.md#runtime-source-adapter-d1)
