# Migrating to the v2 Layout

The v2 layout (specify-cli `0.2.0`) moves the four operator-facing platform artifacts from `.specify/` to the repo root:

| Artifact | v1 path | v2 path |
|---|---|---|
| Platform catalogue | `.specify/registry.yaml` | `registry.yaml` |
| Change plan | `.specify/plan.yaml` | `plan.yaml` |
| Operator brief | `.specify/initiative.md` | `initiative.md` (`change.md` is required by current change commands) |
| API contracts | `.specify/contracts/` | `contracts/` |

`.specify/` continues to hold framework-managed state — `project.yaml`, `context.lock`, `slices/`, `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, and the advisory `plan.lock`. The boundary is "operator artifacts and generated context at root, framework state under `.specify/`". See [Decision Log](../explanation/decision-log.md) for the rationale.

## When you'll see this

The CLI version `0.2.0` and later refuses to read the v1 layout. Any project-aware verb against a project still on v1 errors with a stable `legacy-layout` diagnostic:

```text
$ specify status
error: legacy v1 layout detected; run `specify migrate v2-layout` to upgrade
       ([".specify/registry.yaml", ".specify/plan.yaml"])
```

JSON callers see `error: "legacy-layout"` and exit code `1`.

## The migration

Start with the layout migration:

```bash
cd <your-project>
specify migrate v2-layout
```

The RFC-13 follow-on migration shims have been removed. If the project still predates the slice/change cutover, rename `initiative.md` to `change.md` and move any legacy `.specify/changes/` content to `.specify/slices/` before using current change or slice commands.

The verb is **idempotent**, **atomic per file**, and **refuses to clobber** an existing destination. Re-running on an already-migrated project exits 0 with `nothing to migrate`.

For a preview, use `--dry-run`:

```bash
specify migrate v2-layout --dry-run
```

For machine-readable output (e.g. to thread through CI):

```bash
specify migrate v2-layout --format json
```

The JSON envelope enumerates every checked path with `moved` / `would-move` / `absent-source` / `destination-exists` per row; see [`specify migrate`](../reference/cli/migrate.md) for the wire shape.

## After the move

1. **Commit the migration.** The movers use `fs::rename`; the result is staged exactly as if you had run `git mv`. Inspect with `git status` and commit:

   ```bash
   git add -A
   git commit -m "chore: migrate to v2 layout"
   ```

2. **Run any project-aware verb to confirm.** A subsequent `specify status` (or any other CLI verb) should now succeed.

3. **Update CI / scripts.** Any scaffolded automation that hard-codes `.specify/registry.yaml`, `.specify/plan.yaml`, `.specify/changes/`, or `initiative.md` needs to be updated in lockstep — the CLI no longer reads those current-state paths.

## Multi-repo platforms

A platform-hub repo with `.specify/workspace/<name>/` clones needs each clone migrated separately:

1. Migrate the hub repo first:

   ```bash
   cd <hub-repo>
   specify migrate v2-layout
   ```

   The hub's own `registry.yaml`, `plan.yaml`, and operator brief move to the hub's repo root. The migrate verb **refuses to touch peer clones** under `.specify/workspace/<name>/`.

2. Migrate each peer clone:

   ```bash
  for clone in .specify/workspace/*/; do
    ( cd "$clone" && specify migrate v2-layout )
  done
   ```

3. Each peer clone now has its own root-level `registry.yaml` (typically empty for non-hub projects), `plan.yaml`, and `contracts/` wherever those existed under the clone's old `.specify/`. Rename any remaining `initiative.md` to `change.md` before invoking current change verbs.

4. Push the migrations using the operator's normal per-clone publishing path (`specify workspace push` once everything's committed inside each clone).

## Collisions

If `specify migrate v2-layout` reports a `destination-exists` row, both copies are still on disk. Resolve manually:

1. Diff the two:

   ```bash
   diff -u .specify/registry.yaml registry.yaml
   ```

2. Decide which is canonical. Typically the v1 copy under `.specify/` is the one to keep (the v2 root copy was created by hand or by another script before the migrate ran).

3. Replace the root copy with the v1 copy:

   ```bash
   rm registry.yaml
   ```

4. Re-run `specify migrate v2-layout`. Idempotent — the migrate finishes the move on the next invocation.

## Rolling back

The v1 layout is **read-only** to the CLI starting at `0.2.0`. To roll back:

- Pin `specify-cli` to `0.1.x` in your CI / install scripts.
- Reverse the moves manually:

  ```bash
  mkdir -p .specify
  mv registry.yaml plan.yaml change.md .specify/
  mv contracts .specify/
  mv .specify/slices .specify/changes
  ```

- Commit the reversal.

This is rarely the right answer — the v2 layout is the future direction and the migration is a one-line operator action — but it is mechanically possible.

## See also

- [`specify migrate v2-layout`](../reference/cli/migrate.md) — full CLI reference.
- [Directory Layout](../reference/directory-layout.md) — the v2 shape, with every directory annotated.
- [What's New](../explanation/whats-new.md) — release notes for the v2 layout move.
