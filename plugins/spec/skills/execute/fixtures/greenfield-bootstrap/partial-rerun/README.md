# Partial re-run fixture

Pins the `specify workspace sync` partial-bootstrap recovery path:

- **Precondition:** `.git/` exists but `.specify/project.yaml` is absent
  (prior `specify init` failed)
- **Action:** Re-runs `specify init` + `git add . && git commit --amend`
- **Postcondition:** Healthy workspace slot with `.specify/project.yaml`

The `--amend` ensures the scaffold commit is updated rather than creating
a second commit.
