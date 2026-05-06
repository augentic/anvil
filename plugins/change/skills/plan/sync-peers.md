# Sync peers (step 3b, multi-repo only)

When **`registry.yaml`** exists and declares **more than one** project (`projects.length > 1` in the JSON from `specify registry show --format json`), `/change:plan` enters the **sync-peers** phase between discovery and propose. Single-repo changes (absent registry or `projects.length ≤ 1`) skip this step entirely.

## Normative sequence

1. Shell out to **`specify workspace sync`** from the project root. This materialises `.specify/workspace/<project-name>/` (symlink for local / relative `url:` values; shallow `git clone` / `git fetch` for remotes — see the CLI). Treat a non-zero exit as a hard failure for `/change:plan`.
2. Walk each materialised peer root read-only and author **`.specify/plans/<change-name>/workspace.md`** — the peer inventory the propose brief consumes alongside `discovery.md`.

## `workspace.md` shape (pin for idempotency)

```markdown
# Workspace — <change-name>

## <registry-project-name>

- **Slot:** `.specify/workspace/<registry-project-name>/`
- **Description:** <registry description text from registry.yaml>
- **Schema:** `<schema identifier from registry.yaml>`
- **Materialisation:** `symlink` \| `git-clone` \| `missing` (mirror
  `specify workspace status`).
- **Head:** `<40-char sha or —>` when the slot is a git work tree.
- **Dirty:** `yes` \| `no` \| `—`
- **Specify tree:** one bullet each if present: `plan.yaml`, active
  changes under `changes/`, baseline specs under `specs/`, cached
  schema under `.specify/.cache/` — paths relative to the peer slot.

<!-- one `##` section per registry project, alphabetically by name -->
```

Re-running on an unchanged registry + workspace cache MUST yield byte-identical `workspace.md` (stable ordering throughout).

## Mode interactions

**`--dry-run`.** Do **not** shell `specify workspace sync`; do **not** write `workspace.md`. You MAY print a short preview of what `workspace.md` *would* contain after a real sync, but only to stdout — no writes under `.specify/workspace/` or `.specify/plans/<change-name>/`.

**`--extend`.** Do **not** shell `specify workspace sync` during the sync-peers step — operators refresh clones explicitly between runs. If `.specify/workspace/` already exists, still **rewrite** `workspace.md` from the current on-disk cache (read-only walk) so propose sees an up-to-date peer inventory without an implicit `git fetch`.

Fixture for the inventory shape lives at `fixtures/plan-layer2/workspace.md` (placeholder peer names; copy the heading / bullet contract verbatim).
