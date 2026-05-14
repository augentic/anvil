# Argument resolution (`sources`)

Step 6 of the [per-slice algorithm](per-slice-algorithm.md) turns the plan entry's `sources` field into command-line arguments for `/spec:define`. Scope and delta-targeting intent are carried in the entry's `description` field and inferred by the define skill; the driver does not forward them as separate flags.

- **`sources`** — a list of keys into the plan's top-level `sources` map. Each key resolves to either a local filesystem path or a git URL. The resolved values are handed to `/spec:define` as `source <key>=<path-or-url>` tuples, preserving the key so define's brief pipeline can retain provenance when it hands the value to `/spec:extract` (which inlines a guarded `git clone` snippet for URL values — see the *Cloning a source tree* subsection in [`../analyze/SKILL.md`](../analyze/SKILL.md)) or an analogous plugin.

For every key in the plan entry's `sources` list, look it up in the plan's top-level `sources` map and classify the value:

1. **Key absent from the top-level map** — unresolved reference. The plan is internally inconsistent; this is an `Error::Config`-level halt. Emit a diagnostic naming the offending `(slice, key)` pair, release the driver lock, exit non-zero. This should have been caught earlier by `specify change plan validate` via the `unknown-source` diagnostic, so reaching this branch means either the plan was not validated or it was edited out of band between validation and execution — either way, human triage.
2. **Value is a local filesystem path** (e.g. `/path/to/legacy`) — pass through as-is. The driver does NOT stat the path or verify it exists; `/spec:define` (and downstream `/spec:extract`) are responsible for surfacing a missing-path error with the right phase-level diagnostic.
3. **Value is a git URL** (e.g. `git@github.com:org/service.git` or `https://github.com/…`) — pass through as-is. The driver does NOT clone here. Cloning is the brief's concern (the inlined snippet documented at [`../analyze/SKILL.md` §*Cloning a source tree*](../analyze/SKILL.md)), invoked from inside `/spec:define`'s brief pipeline when a brief needs the source tree materialized. This keeps the clone cache under the phase's control and avoids duplicating the clone logic in the driver.

The path-vs-URL distinction is a content-level classification on the value string; neither `plan.schema.json` nor the plan library distinguishes them (both are validated as `type: string`). The driver emits the tuple as `--source <key>=<value>` unchanged — the classification matters only for the diagnostics rendered in the transcript.

Two authoring pins under `fixtures/field-wiring/` cover the two shapes — `sources-only/` (`/spec:define <name> source monolith=/path/to/legacy`) and `description-driven/` (greenfield or description-inferred entries with no `source` flags) — see [fixtures.md](fixtures.md) for the invocation each one pins.

## Multi-repo source resolution

Under multi-repo routing (per-slice algorithm step 5a active), source paths from the plan's top-level `sources` map are resolved to **absolute filesystem paths** anchored to the initiating repo root before the CWD change. The resolved absolute paths are what gets passed to `/spec:define <name> source <key>=<absolute-path>`. This ensures source paths remain valid regardless of which project clone the driver has `chdir`'d into. Git URLs pass through unchanged.
