# Acceptance Fixtures

> Status: Landed by C05 (contracts smoke runner). Hosts the deterministic materialisation fixtures the [`fixture` backend](../runner/backends/fixture.ts) uses to prove the runner + assertion stage compose end to end without an operator and without a live agent.

Each subdirectory is keyed by **scenario id** and mirrors a scenario's `expected-artifacts:` list under an `expected/` root. The fixture backend copies every file under `expected/` into the temp workspace before the runner-owned `assertions` stage runs.

```text
acceptance/fixtures/
  <scenario-id>/
    expected/
      <relative path 1 from expected-artifacts>
      <relative path 2 from expected-artifacts>
      ...
```

The point of these fixtures is to prove the assertion *plumbing* — they are not goldens for generated content. Keep contents tiny and intentional: a comment that names the assertion id and the path is enough.

## Distinction from C08

C08 introduces a deterministic *workflow* stub backend that drives `define → build → merge` phase outcomes through `specify` lifecycle commands. The `fixture` backend here is a deterministic *materialisation* fixture for one scenario's expected-artifact set; it never touches `.specify/` lifecycle state. The two backends address different problems and live in different files.

## Adding A New Fixture

1. Pick a scenario id whose frontmatter declares an `expected-artifacts:` list.
2. Create `acceptance/fixtures/<scenario-id>/expected/` and add a tiny placeholder file at every declared path.
3. Run the runner with `--scenario <id> --backend fixture --allow-backend-mismatch` (manual scenarios cannot declare `backend: fixture` themselves; the smoke target uses the override flag).
4. Confirm the assertion stage reports each `files-exist` record as `pass`.

If a single fixture's expected-artifacts grow beyond a handful of files or need real generated content, that is a sign the scenario should graduate to the `agent` (C12+) or `recorded` (C15) backend rather than living as a fixture.
