# Hub and registry issues

Use this page when `specify registry` verbs or `specify init --hub` refuse with a hub or registry invariant violation.

## Prerequisites

- A cross-repo program with a `registry.yaml` (either on a hub repo with `project.yaml: hub: true`, or on a code project that has more than one entry in its registry).
- The CLI output naming the failing entry and the invariant code.

## `hub-cannot-be-project`

**Symptom:** `specify registry validate` (or `specify init --hub`) refuses with `hub-cannot-be-project: registry.yaml: projects[<idx>] (<name>).url is `.``.

**Cause:** A registry on a hub repo (`project.yaml: hub: true`) has an entry whose `url` is `.`. The hub topology forbids this -- the hub holds platform state and never appears in its own registry. Code projects always live in their own repos.

**Resolution:** Two paths.

- **Stay on the hub:** remove the entry. `specify registry remove <name>`. Code projects must live in their own repos and be referenced via a remote URL.
- **Convert to platform-as-project:** if the operator actually wants the single-repo shape (the initiating repo is itself a code project), remove `.specify/` and re-run `specify init <adapter>` without `--hub`. See [Platform repo topologies](../../explanation/platform-repo.md).

## `description-missing-multi-repo`

**Symptom:** `specify registry add` or `specify registry validate` refuses with `description-missing-multi-repo` and names the offending entry.

**Cause:** A multi-project registry must declare a `description` on every entry (the description drives `/change:draft`'s assignment step; sparse descriptions force unresolved prompts during planning). The invariant fires when the addition produces a multi-project registry and any existing entry lacks a description, or when validate is run against an already-violating registry.

**Resolution:** Add the missing descriptions. Either re-run `specify registry add` for each existing entry with `--description "..."`, or hand-edit `registry.yaml` and re-run `specify registry validate` to confirm.

```bash
specify registry add <existing-name> \
    --url <existing-url> \
    --adapter <existing-schema> \
    --description "..."
```

`registry add` refuses if the entry already exists; for already-declared entries the operator hand-edits `registry.yaml` and runs `specify registry validate` again.
