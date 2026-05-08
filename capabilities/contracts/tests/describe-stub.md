---
id: contracts-describe-stub
owner: contracts
kind: capability
capability: contracts@v1
backend: stub
entrypoint: /spec:define
stages: [define, build, merge]
isolation: fresh-project
authorship-mode: prose
assertions:
  - files-exist
expected-artifacts:
  - contracts/schemas/create-profile-request.yaml
  - contracts/schemas/profile.yaml
  - contracts/schemas/update-profile-request.yaml
  - contracts/schemas/error-response.yaml
  - contracts/http/profile-api.yaml
stubbed-stages:
  - define
  - build
  - merge
stub-fixtures:
  build: acceptance/fixtures/contracts-describe/expected
---

# Stub Variant Of Contracts Describe

Scenario ID: `contracts-describe-stub`

This scenario is the deterministic-stub twin of [`describe.md`](describe.md). The
prompt and expected artifacts are identical; only the runner backend differs.
The stub backend (C08) drives the slice through real `specify slice {create,
transition, merge run}` commands and materialises the build artifacts from a
fixture so the runner can prove execution mechanics end-to-end without paying
live-agent generation cost.

## Intent

Prove that `make acceptance-stub-smoke` can drive a contracts scenario through
`define → build → merge` in one shot using the C08 deterministic stub backend.
The scenario also exercises the runner's `stub-actions.jsonl` evidence file
and the stub-disclosure block in `summary.md`.

## Workspace

- **Capability:** `contracts@v1`.
- **Project shape:** a single project the stub backend bootstraps from the
  in-repo capability directory via `specify init --name <slice> file://...`.
- **Isolation:** `fresh-project`. The runner creates a temp workspace and
  cleans it on pass.
- **Backend:** `stub` — the runner shells out to `specify` for every
  lifecycle transition; nothing is hand-edited under `.specify/`.

## Inputs

The stub backend writes the slice's `proposal.md`, `specs/main.md`, and
`tasks.md` itself with explicit `STUB:` markers so a reader can tell the
artifacts are fake. The build-stage artifacts are copied verbatim from
`acceptance/fixtures/contracts-describe/expected/` — the same fixture the
C05 `fixture` backend uses, so the on-disk shape stays in lock-step with
the manual scenario.

## Invocation

```text
deno run --allow-read --allow-write --allow-env --allow-run \
  acceptance/runner/main.ts \
  --scenario contracts-describe-stub \
  --backend stub
```

The runner drives `specify slice create`, `specify slice transition`, and
`specify slice merge run` against the resolved binary (set `SPECIFY_BIN` to
override). It does not invoke `/spec:define`, `/spec:build`, or `/spec:merge`
agent entrypoints — those are out of scope for the C08 stub.

## Expected Artifacts

After the merge stage, every path below exists at the workspace root:

- `contracts/schemas/create-profile-request.yaml`
- `contracts/schemas/profile.yaml`
- `contracts/schemas/update-profile-request.yaml`
- `contracts/schemas/error-response.yaml`
- `contracts/http/profile-api.yaml`

## Assertions

- `files-exist`: every path in **Expected Artifacts** exists in the workspace
  after the stub backend materialises the build-stage fixture.

## Negative Expectations

This is a positive coverage scenario; see [`describe.md`](describe.md) for the
boundary checks. The stub backend deliberately does not exercise
`contract-validator-clean` because the fixture's placeholder OpenAPI document
is intentionally minimal — the verifier wiring lands with C13.

## Cleanup

The runner removes the temp workspace and run directory on pass; preserve them
with `--preserve` for inspection. No persistent state is created in the
acceptance repo itself.
