# CLI Contract

The deterministic surface skills depend on. The surviving skill in this repository (`/emery:specify`) shells out to the `emery` binary; it is an ultrathin wrapper over one verb. The v1 workflow verbs and their skills are archived at git tag `v1`.

The CLI itself is built in the in-tree Cargo workspace at the repo root: the `emery-cli` crate (`crates/cli`) owns the grammar, the envelope, and the exit contract as a façade over the transport-neutral `emery-engine` operations. This document captures the verbs skills call, the envelope shape they consume, and pointers to the authoritative wire-contract definitions.

## Rule: all deterministic operations live in the CLI

Skills are ultrathin invoke-and-relay wrappers: they elicit missing arguments, invoke one `emery` verb, and relay its output. Skill markdown must not grow orchestration, synthesis, or validation prose.

When a skill currently does something deterministic in prose (parsing YAML, validating shape, transitioning state), the right fix is to add a CLI verb and have the skill call it. The wrong fix is to make the skill smarter. See [AGENTS.md](../../AGENTS.md).

Never hand-edit `.emery/` state (the revision store); never `mkdir -p .emery/...`. Route through the CLI — it enforces the legal set of states and validates inputs in one place for humans, agents, and CI alike.

## Verb tree

- `emery specify <adapter>... [--description <adapter>=<text>] [--config [<path>]]` — the spec generator: resolve the sources named on the invocation (a project-relative local component loads through the deployment loader, read fresh each run; an exact package reference fetches from the binding's `registry` override or the compiled-in default endpoint; either load's optional `digest` pin is verified host-side and the resolved digest rides the success envelope), extract, reconcile, synthesise, and commit `spec.md` / `design.md` as one revision, atomically swapping the current revision id. `--config` without a value selects the project-relative `emery.toml`; a run naming no bindings at all discovers the project-root `emery.toml` as a fallback, never merged with argv bindings. The binding list is per-run input, never persisted. Invoked without a source — and with nothing to discover — it exits `1` with `specify-source-required`; mixing `--config` with argv bindings, or naming a filesystem path outside the `.` project preopen, exits `1` with `bad_request`.
- `emery show <spec|design>` — print a reviewable document of the current revision; text stdout is the document body alone. Before any commit it fails `spec-not-generated` (exit `2`).
- `emery completions <shell>` — auto-derived shell completions over the live clap surface.

## JSON envelope

Every CLI verb that skills consume emits a stable **flat body**: the command-specific fields at the top level of a single JSON object. On success the body is exactly that — there is no `ok` discriminant, no `data` wrapper around the payload, and no top-level envelope-version stamp. On failure the flat object carries three top-level keys: `error` (a discriminant string: kebab-case for the three recovery codes, snake_case for Omnia defaults), `message` (a humanised one-liner), and `exit-code` (the integer the binary returns). Skills invoked with `--format json` parse the body and branch on the `error` field rather than on stdout text.

Stream roles are part of the contract: the semantic result body (text or JSON) is stdout; the failure body and live host tracing are stderr. The failure body carries no styling in either format; colour policy belongs to the host tracing layer alone. Host tracing is selected by the reserved host log flags, peeled from argv before the guest sees it: bare invocations default to INFO progress, `--quiet` turns tracing off, and `--debug` adds backend debug tracing (both flags win over any ambient `RUST_LOG`). Skills follow the plugin rule's tracing contract and relay the semantic result once without repeating tracing lines.

The canonical envelope shapes live in [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md). SKILL.md bodies **link** to that reference rather than embedding envelope JSON inline (house style applied in review).

The `error` discriminants are part of the public contract that skills and tests grep for. Examples skills handle today:

- `specify-source-required` — `emery specify` without a source binding and with no project-root `emery.toml` to discover.
- `spec-not-generated` — `emery show` before any revision is committed.
- `unsupported-version` — an adapter's declared minimum `emery-version` is newer than the running binary.
- `refused` — the loader rejected the request: a mismatched or malformed `digest` pin, an invalid component, a missing source-seam export, or a location kind this deployment does not serve; the message names which.
- `unavailable` — the deployment's acquirer could not produce a registry package (network, endpoint, or a missing exact version); check connectivity and the binding's `registry` override.

## Exit codes

The CLI uses the Omnia 1:1 exit map. The authoritative definition lives in [`AGENTS.md`](../../AGENTS.md#exit-codes). Summary for skills:

| Code | Name | Skills see it on |
|---|---|---|
| `0` | `EXIT_SUCCESS` | Command succeeded; parse the body. |
| `1` | `BadRequest` | Operator or input refusal (`specify-source-required`, `unsupported-version`, the loader's kebab-case refusals — `refused` and `already-active` — or the Omnia default `bad_request`). Parse the top-level `error` discriminant; on `unsupported-version`, tell the operator to update the installed binary through its install channel. |
| `2` | `NotFound` | Missing resource (`spec-not-generated`, or the Omnia default `not_found`). Clap usage and unknown-verb also exit 2 (framework). |
| `3` | `ServerError` | Unclassified default: I/O, storage (`server_error`, or the loader's `internal`). |
| `4` | `BadGateway` | Upstream, model, or component-acquisition failure (`bad_gateway`, the loader's `unavailable`). |

Skills should branch on the exit code first (success vs failure class) and on the three kebab recovery discriminants second (`specify-source-required`, `unsupported-version`, `spec-not-generated`). Other failures share the Omnia snake_case default for that class (`bad_request`, `not_found`, `server_error`, `bad_gateway`). New exit codes are not invented by skills or the CLI.

## Cross-references

- [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md) — canonical envelope shapes per verb.
- [`AGENTS.md`](../../AGENTS.md) — authoritative source for exit codes, Omnia error classes, `error` discriminants, and CLI architecture.
