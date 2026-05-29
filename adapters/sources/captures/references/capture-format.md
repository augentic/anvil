# Runtime capture wire format

The `captures` source adapter consumes a read-only capture tree under `$SOURCE_DIR`. `/capture:wiretapper` writes this layout; operators with a non-conforming tree adapt the directory or write a thin wrapper adapter — v1 does not invent a new format.

## Directory layout

```text
$SOURCE_DIR/
└── tests/data/replays/
    ├── <handler>/                # one subdirectory per captured handler (lead grain)
    │   ├── <scenario>.json       # TestDef-style capture (claim grain)
    │   └── INSTRUCTIONS.md       # optional per-handler hint material — not evidence
    └── samples/                  # optional shared bulk payloads — not captures, not handlers
```

- **`<handler>/`** — kebab-case directory name becomes the lead `id` at survey time.
- **`<scenario>.json`** — one scenario per file; extract emits one `kind: example` claim per file.
- **`samples/`** — shared bulk data referenced by captures via `@samples/` paths. Not a handler directory; survey skips it.
- **`INSTRUCTIONS.md`** — optional operator hints for Omnia test generation. Read for surface-naming context if needed; do not turn prose into Evidence claims. Test-harness semantics live in [`adapters/targets/omnia/references/replay-fixtures.md`](../../../targets/omnia/references/replay-fixtures.md).

## TestDef-style scenario files

Each `<scenario>.json` records one observed handler invocation. Internal field shapes depend on the handler under test; the top-level envelope is:

```json
{
    "setup": { ... },
    "input": "<raw value>",
    "params": { "delay": 0 },
    "http_requests": [ { "path": "/api/x", "response": { "body": [...] } } ],
    "output": { "success": [...] } | { "failure": { "BadRequest": { "code": "...", "description": "..." } } }
}
```

### Behavioural fields (extract reads these)

- **`input`** — raw input to the handler (string for query/path, object for JSON body, etc.). Typically matches the handler's request object shape.
- **`params`** — optional test parameters (e.g. timestamp delay, normalisation flags). Consistent across scenarios for a handler.
- **`http_requests`** — optional observed or mocked outbound HTTP responses keyed by path/method for handlers that call `HttpRequest` internally.
- **`output`** — optional observed success (array of events/data) or failure (error variant and code/description). Side effects (published messages, state writes) belong here when the capture records them.

All fields other than **`input`** are optional. Captures may record scenarios where processing is skipped or intermediate steps fail.

### Non-evidence fields (extract ignores for claims)

- **`setup`** — optional MockProvider configuration for test replay. Not behavioural evidence; Omnia test generation consumes it per [`replay-fixtures.md`](../../../targets/omnia/references/replay-fixtures.md).

### `@samples/` file references

Values prefixed with `@samples/` resolve relative to `tests/data/replays/`. Example: `"@samples/fleet-data.json"` → `tests/data/replays/samples/fleet-data.json`. The adapter knows these paths exist for citation in `path` fields but does not treat sample files as scenario captures.

## What is not a capture

| Path | Role |
|---|---|
| `samples/*.json` | Shared bulk datasets — not scenario files, not handler directories |
| `INSTRUCTIONS.md` | Operator hints for test generation — not Evidence |
| Directories named `.` / `_` prefix | Skipped at survey time |

## See also

- [`extraction-mapping.md`](extraction-mapping.md) — capture JSON → Evidence claim field mapping
- [`../briefs/survey.md`](../briefs/survey.md) — handler-grain lead survey
- [`../briefs/extract.md`](../briefs/extract.md) — `kind: example` claim emission
- Test-harness docs are **per-target** — Omnia: [`replay-fixtures.md`](../../../targets/omnia/references/replay-fixtures.md); hook contract: [`../../../shared/target-hooks/replay/`](../../../shared/target-hooks/replay/)
