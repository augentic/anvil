# Fixture → Evidence extraction mapping

Maps runtime fixture JSON fields to `kind: example` claim fields emitted by the `runtime-fixtures` extract brief. Full procedure and caps live in [`../briefs/extract.md`](../briefs/extract.md).

## Field mapping

| Fixture source | Evidence claim field | Rule |
|---|---|---|
| File path under `$SOURCE_DIR` | `path` | Relative path; no `#L` anchors — the whole file is the citation |
| Raw file bytes | `fixture-digest` | `sha256:` prefix over exact bytes (no re-serialisation) |
| `<handler>/<stem>.json` | `claim-id` | `<candidate-id>.<stem>` kebab-case (mechanical derivation) |
| `input` + inferred HTTP surface | `input.method`, `input.route`, `input.body` | Quote observed shapes verbatim from fixture |
| `params` | fold into `input` or omit | Include when they affect observed behaviour |
| `http_requests` | `input` outbound context or omit | Structural summary only when relevant to the scenario |
| `output.success` / `output.failure` | `output.status`, `output.body` | HTTP status or channel equivalent |
| Published messages in `output` | `output.side-effects[]` | `kind`, `topic`, payload **shape** — not raw bulk payloads |
| Scenario observation | `statement` | Single-line summary; not a JSON dump |
| Serialised claim body > 64 KiB | omit `input` / `output` | Required fields + digest + path only |

## Minimal example

Fixture `tests/data/replay/user-registration/happy.json`:

```json
{
    "input": { "email": "bob@example.com", "password-hash": "$argon2..." },
    "output": {
        "status": 201,
        "side-effects": [{ "kind": "message-pub", "topic": "user.created" }]
    }
}
```

Extracted claim (candidate `user-registration`, source key `runtime`):

```yaml
  - kind: example
    claim-id: user-registration.happy
    path: tests/data/replay/user-registration/happy.json
    fixture-digest: sha256:7a2b...
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
```

## See also

- [`fixture-format.md`](fixture-format.md) — on-disk wire format
- [`../briefs/extract.md`](../briefs/extract.md) — 64 KiB cap, determinism, anti-patterns
