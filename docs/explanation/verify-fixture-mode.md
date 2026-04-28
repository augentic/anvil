# Fixture-backed verification mode

> **Status:** design — implementation pending RFC-9 §4D follow-up (`rfc9-4d2-impl`).
>
> **Scope of this note.** Specify `/spec:verify`'s default mode performs requirement-vs-source drift detection. RFC-9 §4D adds a second mode that replays captured fixtures against the live, deployed service and reports response drift. This note specifies the design so that a follow-up implementation change can land it without re-deriving the model.

## Abstract

Fixture-backed verification (henceforth **fixture mode**) is a mode of [`/spec:verify`](../../plugins/spec/skills/verify/SKILL.md) that:

1. Loads a directory of [TestDef-style fixtures](../../plugins/rt/skills/replay-writer/references/fixture-format.md) — the same JSON shape consumed by `replay-writer` and the same shape Specify already standardises in `tests/data/replay/`.
2. Resolves a transport binding for each fixture (HTTP method/path or Kafka topic) from a sidecar configuration.
3. Replays each fixture against a live target (typically the migrated service) and captures the actual response or side-effect.
4. Diffs the actual against the fixture's recorded `output`, applying a configurable tolerance policy.
5. Emits a verify-level **fixture drift report** alongside the existing requirement drift report.

Fixture mode is opt-in: the operator passes `--fixtures <dir>` to `/spec:verify`. The default mode is unchanged.

## Why this mode

The existing migration toolkit stops at the migration boundary:

- `wiretapper` captures a `{appName}.wiretap.json` file from the running legacy service (request → response, plus side-effects like outbound HTTP and Kafka publishes).
- `replay-writer` consumes a curated subset of those captures, placed under `tests/data/replay/`, and wires them into `cargo test` as in-process unit tests against a `MockProvider`.

That gives a one-shot integration test at migration time, but it does not give an **ongoing regression guard**: once the migrated service is live, drift between live behaviour and the captured baseline is invisible. Examples of drift that matter:

- A producer dependency adds a field that the new service does not propagate, even though the old one did.
- A new code path silently changes an error code from `bad_time` to `invalid_time`.
- Kafka publishes acquire a new key shape that downstream consumers reject.
- A handler's latency-shaped behaviour (delay buckets, retry decisions) flips in a way unit tests with mock time cannot catch.

Fixture mode closes the loop by treating the captured fixtures as a **golden** behavioural reference and verifying live drift on demand, e.g. as a post-deploy gate or a periodic check.

## Inputs

### 1. Fixture directory

The primary input is a directory in the canonical replay layout:

```text
<fixtures-dir>/
├── INSTRUCTIONS.md          # (optional) freeform guidance, see replay-writer fixture-format.md
├── transport.yaml           # (NEW for fixture mode) maps fixtures to live transport bindings
├── tolerances.yaml          # (optional, NEW) tolerance policy; defaults applied when absent
├── samples/                 # (optional) shared bulk data referenced via @samples/...
│   └── *.json
└── *.json                   # TestDef-style fixtures (one scenario per file)
```

The fixture file format is the [TestDef format](../../plugins/rt/skills/replay-writer/references/fixture-format.md) verbatim: `setup` / `input` / `params` / `http_requests` / `output`. Fixture mode reuses this format so a single curated directory can drive both the in-process replay-writer tests and live verification.

The fixture file shape leaves one piece of information **handler-implicit**: the transport binding (HTTP method/path, Kafka topic). In `replay-writer` that binding is supplied by the test harness (the test file `include_bytes!`s the fixture and dispatches into a known handler). For live replay, the binding is not implicit — a runner must know which URL to POST to or which topic to publish on. Fixture mode therefore introduces `transport.yaml`.

### 2. `transport.yaml`

`transport.yaml` is a sidecar that maps fixture files (or globs) to a transport binding. The simplest shape:

```yaml
version: 1

defaults:
  base_url: "${SPECIFY_VERIFY_BASE_URL}"        # required for HTTP bindings
  timeout: 5s                                   # per-request timeout

bindings:
  # File-pattern → transport binding. First match wins.
  - match: "fleet/*.json"
    transport:
      kind: http
      method: GET
      path: /api/v1/fleet
      query_from: input                          # the fixture's `input` is treated as querystring
  - match: "trains/cco-*.json"
    transport:
      kind: http
      method: POST
      path: /api/v1/cco
      body_from: input                           # the fixture's `input` is the request body
      content_type: application/xml
  - match: "kafka/cco-events-*.json"
    transport:
      kind: kafka
      topic: cco-events
      key_from: input.sequence                   # JSON Pointer or dotted path into `input`
      value_from: input
```

Notes for the implementer:

- `base_url`, `topic`, `path`, etc. accept `${ENV_VAR}` interpolation so fixtures stay environment-agnostic.
- `query_from` / `body_from` / `value_from` accept literal `input` (use the fixture's `input` field whole) or a JSON Pointer / dotted accessor when the binding requires only a slice.
- `kind: http` MUST resolve to a method/path/headers triple after interpolation. `kind: kafka` MUST resolve to a topic, value, and optional key.
- The runner refuses to start if any fixture in the directory does not match exactly one binding. Unmatched fixtures are surfaced as a `MISSING_BINDING` failure mode (see *Failure modes*).
- `transport.yaml` SHOULD live in the fixture directory. The `--fixtures <dir>` flag implicitly looks for `<dir>/transport.yaml`. An override (`--transport <path>`) is a possible future extension and is left for the implementation change.

### 3. `tolerances.yaml`

`tolerances.yaml` configures how the runner compares an actual response against a fixture's expected `output`. Default tolerance is **strict equality** with **additional fields rejected**. The config relaxes that policy.

```yaml
version: 1

diff:
  additional_fields: allow        # allow | warn | reject (default: reject)
  missing_fields: reject          # reject | allow         (default: reject)
  numeric_epsilon: 0              # absolute tolerance for f64 comparisons (default: 0)

rules:
  # Rules apply in order; first match wins per JSON Pointer or path expression.

  - path: "$..timestamp"
    kind: timestamp
    window: 5s                    # accept any RFC-3339 string within ±window of the recorded value

  - path: "$..receivedAt"
    kind: timestamp
    window: 60s

  - path: "$..id"
    kind: redact                  # accept any value at this path (used for generated UUIDs etc.)

  - path: "$.success[*]"
    kind: array_unordered
    key: messageData.timestamp    # match elements by key, not position

  - path: "$..version"
    kind: ignore                  # remove the path from both sides before diffing

  - path: "$..coordinates"
    kind: numeric
    epsilon: 1e-6                 # per-path numeric tolerance overrides global numeric_epsilon
```

Tolerance dimensions the implementation must support:

| Kind | Meaning |
|------|---------|
| `timestamp` | Path holds an RFC-3339 / ISO-8601 string. Accept actual within `window` of expected. Window defaults to `0s` if omitted (i.e. exact match required). |
| `redact` | Path holds a generated value (UUID, autoincrement). Accept any non-null value of the same JSON type. |
| `ignore` | Strip the path from both expected and actual before diffing. |
| `array_unordered` | Match array elements by `key` (a JSON Pointer / dotted accessor) rather than position. |
| `numeric` | Compare numeric values within an absolute or relative epsilon (epsilon is absolute by default; `relative_epsilon` is the optional ratio variant). |
| `regex` | Path holds a string matching a recorded regex (e.g. for opaque session tokens with known shape). |

Implementation notes:

- `path` is a JSON Pointer (`/$..foo`) or a `$.foo[*].bar`-style expression. The implementation MAY pick either syntax (or both); the design constrains *that the language is path-based*, not which dialect.
- Rules apply on top of the canonical diff. They never *invent* fields — `additional_fields: allow` is the only knob that loosens the default rejection of unexpected keys in the actual.
- A rule with no `path` applies globally (e.g. `kind: ignore` with no path is invalid; use `additional_fields: allow` instead).
- Defaults: when `tolerances.yaml` is absent, fixture mode applies `{additional_fields: reject, missing_fields: reject, numeric_epsilon: 0}` and no path rules. This is intentionally strict so silent loosening cannot creep in.

### Worked example

Given a fixture (truncated for clarity):

```json
{
  "input": "<CCO ...>...</CCO>",
  "params": { "delay": 9 },
  "output": {
    "success": [
      {
        "eventType": "Location",
        "receivedAt": "2025-10-07T11:00:00.000Z",
        "messageData": { "timestamp": "2025-10-08T07:00:46.961Z" },
        "remoteData": { "externalId": "AMP1074" },
        "locationData": { "latitude": -36.84448, "longitude": 174.76915, "speed": 0 }
      }
    ]
  }
}
```

…and the live response containing the same payload but with a fresh `receivedAt`, a `traceId` field that is not in the fixture, and `latitude` / `longitude` differing by 1e-7, fixture mode reports PASS when `tolerances.yaml` declares:

```yaml
diff:
  additional_fields: allow
rules:
  - path: "$..receivedAt"
    kind: timestamp
    window: 60s
  - path: "$..coordinates"
    kind: numeric
    epsilon: 1e-6
```

…and reports DRIFTED otherwise.

### 4. Service endpoint configuration

Endpoint configuration is supplied via `transport.yaml:defaults` plus environment variables. Fixture mode does **not** introduce a new credential store; if a deployed service requires authentication, the operator supplies it via headers in `transport.yaml`:

```yaml
defaults:
  base_url: "${SPECIFY_VERIFY_BASE_URL}"
  headers:
    Authorization: "Bearer ${SPECIFY_VERIFY_TOKEN}"
```

Kafka transports inherit the same env-interpolation rule for broker URLs.

## Algorithm

```text
verify --fixtures <dir>:
    1. Load <dir>/transport.yaml (required); fail-fast if absent or invalid.
    2. Load <dir>/tolerances.yaml (optional); apply defaults if absent.
    3. Discover fixtures: every <dir>/**/*.json that is not under samples/.
    4. For each fixture F:
       a. Resolve F → binding via transport.yaml. If unmatched → MISSING_BINDING.
       b. Render the request: substitute F.input into the binding's body/query/value template.
       c. Apply F.setup (config overrides, etc.) — see "setup" below.
       d. Send the request to the live target. Capture (status, headers, body) for HTTP
          or (publish-success, broker-ack) plus side-effect topic captures for Kafka.
       e. Diff captured-actual against F.output:
          - For HTTP success fixtures (output.success): diff actual response body.
          - For HTTP failure fixtures (output.failure): assert error variant + code + description.
          - For Kafka fixtures: diff observed published message against F.output's expected
            published payload, side-effects-first.
       f. Apply tolerance rules; classify as PASS / DRIFTED / FAILED / MISSING / SKIPPED.
    5. Aggregate results into a fixture drift report.
    6. Exit 0 if all PASS or SKIPPED; exit 1 otherwise. Match existing /spec:verify exit semantics.
```

### `setup` block handling

Replay-writer fixtures may carry a `setup` block that pre-loads the `MockProvider`. In live replay there is no MockProvider — the live service has its own state. The runner therefore handles `setup` defensively:

- **`setup.config` values** are *advisory*: emit a diagnostic noting that the fixture expects the service to be configured a certain way (e.g. `CAPACITY_OVERWRITE`) and require the operator to assert this externally. The runner does not attempt to mutate live config.
- **`setup.data`, `setup.seed_cache`, `setup.state_store`, `setup.table_store`** are not directly applicable. The runner emits a `SETUP_NOT_LIVE_REPLAYABLE` warning and records it in the diagnostic, but still attempts the replay (the live service's actual state determines the actual response).
- The implementation MAY add a future `--seed <script>` flag that lets an operator script live state mutations before replay; this is out of scope for the design.

### `http_requests` block handling

`http_requests` describes outbound calls a handler made on the legacy side. In live replay, those calls go to whatever endpoints the live service is configured to call — fixture mode does **not** intercept or stub them. The runner records, in the diagnostic, that the fixture expected N specific outbound calls; if the live service produced different outbound traffic this is a known drift dimension that requires a service-side observability hook to verify (out of scope for the initial implementation, captured in *Open questions*).

## Diagnostics

Fixture mode extends `/spec:verify`'s output with a **Fixture Drift** section, parallel to the existing **Drift Report** but reporting fixture outcomes rather than requirement coverage.

```text
## Drift Report

### user-auth
| Status      | ID      | Requirement       | Detail |
|-------------|---------|-------------------|--------|
| COVERED     | REQ-001 | Password login    | |
| COVERED     | REQ-002 | Session timeout   | |

## Fixture Drift Report

### user-auth (fixtures: tests/data/replay/)
| Status   | Fixture                       | Binding              | Detail |
|----------|-------------------------------|----------------------|--------|
| PASS     | login-happy-path.json         | POST /api/v1/login   | |
| DRIFTED  | login-with-mfa.json           | POST /api/v1/login   | response.factors[0].kind: expected "totp" got "totp_v2" |
| FAILED   | login-rate-limited.json       | POST /api/v1/login   | live status 500 (expected 429) |
| MISSING  | login-network-down.json       | POST /api/v1/login   | request errored: connection refused |
| SKIPPED  | login-cache-miss.json         | POST /api/v1/login   | setup.seed_cache not live-replayable (warning) |

### Summary
- 2/3 requirements covered    (default mode)
- 1 requirement drifted       (default mode)
- 5 fixtures executed
- 1 PASS, 1 DRIFTED, 1 FAILED, 1 MISSING, 1 SKIPPED

### Suggested Actions
- DRIFTED login-with-mfa: response shape changed; either update the fixture (re-capture
  with `wiretapper`) or fix the live service to restore the expected shape.
- FAILED login-rate-limited: live service returned 500; the rate-limit handler may
  have regressed.
- MISSING login-network-down: request never reached the live service; check
  base_url/headers in transport.yaml.
```

Status semantics:

| Status | Meaning | Exit code contribution |
|--------|---------|------------------------|
| `PASS` | Live response matched the fixture's `output` under tolerance. | 0 |
| `DRIFTED` | Live response reached the runner but differed under tolerance. | 1 |
| `FAILED` | Transport succeeded but the response asserted the wrong shape (e.g. wrong status, wrong error variant). | 1 |
| `MISSING` | The runner could not reach the live target (network error, missing binding, malformed fixture). | 1 |
| `SKIPPED` | The fixture was discovered but intentionally not replayed (e.g. `setup` requires non-live state). | 0 |

The `DRIFTED` detail SHOULD include either:

- a JSON Pointer + expected/actual value pair (for small diffs), or
- a unified-diff-style block (for large diffs), capped at a configurable line count.

## Failure modes

| Failure | Cause | Reporting |
|---------|-------|-----------|
| `MISSING_FIXTURES_DIR` | `--fixtures <dir>` does not exist or is not a directory. | Single line, exit 1. |
| `MISSING_TRANSPORT_YAML` | `<dir>/transport.yaml` not found. | Diagnostic with a remediation pointing at this design note. Exit 1. |
| `INVALID_TRANSPORT_YAML` | YAML parse error or schema mismatch. | Validation diagnostic with line numbers. Exit 1. |
| `MISSING_BINDING` | A fixture matches no binding rule. | Per-fixture entry in the drift report; the runner does not abort other fixtures. |
| `UNREACHABLE_TARGET` | Network error against `base_url`. | All fixtures using that base_url report `MISSING`. The runner emits a single header diagnostic noting `base_url=<...>` is unreachable. |
| `SCHEMA_INCOMPATIBLE` | The live response is not parseable as JSON (e.g. HTML error page returned). | Per-fixture `FAILED` entry with the first 200 bytes of the response in the detail. |
| `TIMEOUT` | Request exceeded `transport.timeout`. | Per-fixture `MISSING` with the timeout value. |
| `SETUP_NOT_LIVE_REPLAYABLE` | Fixture `setup.{data,seed_cache,...}` cannot be applied to live state. | Per-fixture `SKIPPED` (preserves green CI when the operator deliberately keeps such fixtures around for unit-test parity). The runner's exit code is unaffected. |

The runner is fail-soft per fixture: a single bad fixture does not abort the rest of the run.

## Operator workflow

The end-to-end use case is post-deployment regression checking:

```sh
# 1. Capture fixtures from the legacy service (one-shot).
WIRETAP_ENABLED=true ./legacy-server &
# replay traffic against legacy …
# kill legacy; collect <appName>.wiretap.json

# 2. Curate fixtures into tests/data/replay/ (manual, or via a future importer).
#    Author transport.yaml mapping fixtures → live bindings.
#    Optionally author tolerances.yaml for known noisy paths (timestamps, IDs).

# 3. Deploy the migrated service (via existing Specify workflow).

# 4. Verify against captured fixtures.
SPECIFY_VERIFY_BASE_URL=https://migrated.example.com \
  /spec:verify --fixtures crates/cco_handler/tests/data/replay/

# Expected: all fixtures PASS. Any DRIFTED/FAILED is real drift to triage.
```

Fixture mode is callable directly (as above) and is also the natural target for a post-merge guard in CI.

## Implementation surface

The minimum viable implementation is **skill-only**: extend `/spec:verify`'s instructions to recognise `--fixtures <dir>` and emit the fixture drift report. The agent reads `transport.yaml`, performs the live replay (via Shell tool calls to `curl` or a future helper), diffs in-process, and renders the report.

This keeps the blast radius small. A native `specify verify --fixtures` CLI is a candidate evolution once:

- The diff engine accumulates enough complexity (path expressions, tolerance rules) to benefit from a typed implementation.
- Performance becomes a concern for large fixture sets (>100 fixtures).
- A non-agent CI runner needs to invoke the same logic without an agent in the loop.

When that day comes, the CLI surface is:

```text
specify verify fixtures <dir>
  [--transport <path>]      # override <dir>/transport.yaml
  [--tolerances <path>]     # override <dir>/tolerances.yaml
  [--base-url <url>]        # overrides transport.yaml:defaults.base_url
  [--filter <glob>]         # only replay fixtures matching <glob>
  [--json]                  # emit drift report as JSON
```

`specify verify fixtures` would slot under the existing `specify verify` group (which today does not yet exist as a CLI command — `/spec:verify` is skill-only; see the implementation note in [`verify/SKILL.md`](../../plugins/spec/skills/verify/SKILL.md)). A future programmatic `specify verify` CLI would expose `requirements` and `fixtures` as sibling subcommands.

## Open questions

These are the items the design note explicitly does **not** resolve. The follow-up implementation change (`rfc9-4d2-impl`) MUST close each one.

1. **Path expression dialect.** JSON Pointer (RFC 6901) and JSONPath are both viable for `tolerances.yaml:rules[].path`. JSON Pointer is unambiguous and stdlib-supported in most languages but lacks `..` recursive descent (forces verbose paths). JSONPath has wider semantic surface but no single canonical spec. **Recommendation pending implementation:** start with a strict subset of JSONPath (`$.`, `$..`, `[*]`, `[<int>]`, `[<key>]`) and reject everything else.
2. **Side-effect verification for outbound calls.** Fixtures' `http_requests` and Kafka `output.success` describe expected outbound side-effects. Verifying these live requires an observability hook (the migrated service must report outbound calls and publishes back to the runner). Out of scope for the initial implementation; captured here so a later change can land it.
3. **State-mutation seeding.** Live replay treats `setup.{data,seed_cache,state_store,table_store}` as advisory and skips fixtures that depend on them. A future `--seed <script>` flag could let operators run a state-mutation script before replay (e.g. seed a test tenant). Design intentionally deferred.
4. **CI exit-code semantics.** Should `SKIPPED` fixtures fail CI when the operator wants strict coverage? A `--strict-coverage` flag could elevate `SKIPPED` to a non-zero exit. Not designed; left for the implementation change.
5. **Fixture freshness signal.** When fixtures are months old and the service has legitimately evolved, `DRIFTED` floods the report. A "freshness" timestamp on `transport.yaml` (`captured_at: 2025-10-08`) plus a CLI flag (`--max-age 30d`) could surface the staleness explicitly. Deferred.
6. **Importer for `wiretap.json` → `tests/data/replay/`.** Today the curation is manual. A `specify rt fixtures import <wiretap.json>` verb would convert wiretapper output into the canonical replay layout, including a draft `transport.yaml` derived from wiretapper's `METHOD path` / `topic:Name` keys. Out of scope for `rfc9-4d2-impl`; tracked as a candidate follow-up under the RT plugin.
7. **Concurrency.** Should fixtures replay in parallel? The default should be sequential (deterministic, cheap to reason about). A future `--concurrency <n>` flag is a likely follow-up. Implementer choice.
8. **Default tolerance heuristics.** Some sites would benefit from a tolerance preset for "ignore all RFC-3339 timestamp-shaped strings" without hand-listing every path. Whether to ship such presets, or keep tolerances strictly opt-in, is not decided. Recommendation: opt-in only — presets risk silent passing of real drift.
9. **Where does the report live?** Fixture mode currently only prints the report. Persisting it (e.g. as a `.specify/verify/<timestamp>.md` artefact) would let multi-run trend analysis grow on top. Deferred.

## Cross-links

- [`/spec:verify` skill](../../plugins/spec/skills/verify/SKILL.md) — the skill that fixture mode extends.
- [Replay-writer fixture format](../../plugins/rt/skills/replay-writer/references/fixture-format.md) — TestDef shape consumed by fixture mode.
- [Replay-writer skill](../../plugins/rt/skills/replay-writer/SKILL.md) — fixture mode shares the on-disk layout but targets a different consumer (live service vs `MockProvider`).
- [Wiretapper skill](../../plugins/rt/skills/wiretapper/SKILL.md) — the upstream capture step. Wiretapper's `{appName}.wiretap.json` is the source data that gets curated into `tests/data/replay/`.
- [Wiretapper design](../../plugins/rt/skills/wiretapper/references/design.md) — handler-key conventions (`METHOD path`, `topic:Name`) reused by `transport.yaml`.
- [RFC-9 §4D](../../rfcs/rfc-9-platform.md) — the active RFC item this design serves.
- [RFC-2 §Future / Migration Mode](../../rfcs/archive/rfc-2-execution.md) — the original deferred entry that RFC-9 §4D resurrects.
