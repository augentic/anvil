# Repository Structure

## Artifact Boundaries

Specify artifacts have separate responsibilities:

- **`proposal.md`** -- Why the change exists and what is in scope
- **`spec.md`** -- Behavioral requirements, scenarios, error conditions, optional metrics
- **`design.md`** -- Domain model, APIs, integrations, configuration, technical logic
- **`tasks.md`** -- Implementation sequencing only

Behavioral specs should remain platform-neutral. Omnia trait selection, guest wiring, WASM translation, and Crux type system design belong in specialist skills and references.

## File Locations

In downstream consumer projects:

- **Crates**: `$PROJECT_DIR/crates/<crate_name>/`
- **Metrics**: `$PROJECT_DIR/.metrics.json` when tracking is enabled

In this repository:

- **Working artifacts**: `$PROJECT_DIR/.specify/changes/<change-name>/`
- **Baseline specs**: `$PROJECT_DIR/.specify/specs/`
- **Archived changes** (merged or dropped): `$PROJECT_DIR/.specify/archive/YYYY-MM-DD-<change-name>/`

## Skill / CLI responsibility split

Phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`)
are agent-driven orchestrators; every deterministic operation they need —
`.metadata.yaml` reads and writes, lifecycle transitions, schema/brief
resolution, pipeline topology, artifact-completion checks, spec-merge
preview, baseline conflict detection, delta merge, coherence validation,
archive move — is delegated to the `specify` CLI.

| Concern                                   | CLI surface                                                    |
|-------------------------------------------|----------------------------------------------------------------|
| Scaffold `.specify/` and config           | `specify init`                                                 |
| List / inspect changes                    | `specify status`, `specify change list`, `specify change status` |
| Create a change (with kebab-case check)   | `specify change create <name> [--if-exists ...]`               |
| Lifecycle transitions (incl. timestamps)  | `specify change transition <name> <target>`                    |
| `touched_specs` scan / explicit set       | `specify change touched-specs <name> {--scan | --set ...}`     |
| Overlap across active changes             | `specify change overlap <name>`                                |
| Archive / drop (with reason)              | `specify change archive <name>` / `specify change drop <name>` |
| Schema resolution + brief pipeline        | `specify schema {resolve, check, pipeline}`                    |
| Structural + semantic validation          | `specify validate <change-dir>`                                |
| Task progress / checkbox flip             | `specify task {progress, mark}`                                |
| Merge preview (no-write)                  | `specify spec preview <change-dir>`                            |
| Baseline drift detection                  | `specify spec conflict-check <change-dir>`                     |
| Commit merge + archive                    | `specify merge <change-dir>`                                   |

Skills never `mkdir -p .specify/...`, `mv ... .specify/archive/...`, or
hand-edit `.metadata.yaml`. All such writes flow through the CLI, which
enforces the legal lifecycle transitions and `.metadata.yaml` shape in a
single place.
