# Test Fixtures

Each subdirectory is keyed by **scenario id** and mirrors a scenario's
`expected-artifacts:` list under an `expected/` root. These fixtures keep
owner-local scenario metadata resolvable under static validation.

```text
tests/fixtures/
  <scenario-id>/
    expected/
      <relative path 1 from expected-artifacts>
      <relative path 2 from expected-artifacts>
      ...
```

The point of these fixtures is to prove that expected paths resolve; they are
not goldens for generated content. Keep contents tiny and intentional: a comment
that names the assertion id and the path is enough.

## Adding A New Fixture

1. Pick a scenario id whose frontmatter declares an `expected-artifacts:` list.
2. Create `tests/fixtures/<scenario-id>/expected/` and add a tiny placeholder
   file at every declared path.
3. Run `make checks` to confirm the frontmatter and fixture references resolve.

If a fixture grows beyond a handful of placeholder paths or needs real generated
content, prefer a focused test for that behavior instead of expanding the
fixture set.
