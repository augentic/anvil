# CLI Contract

The deterministic surface skills depend on. The surviving skill in this repository (`/emery:specify`) shells out to the `emery` binary; it is an ultrathin wrapper over one verb. The v1 workflow verbs and their skills are archived at git tag `v1` (ADR-0008).

The CLI itself is built in the in-tree Cargo workspace at the repo root. This document captures the verbs skills call, the envelope shape they consume, and pointers to the authoritative wire-contract definitions.

## Rule: all deterministic operations live in the CLI

Skills are ultrathin invoke-and-relay wrappers: they elicit missing arguments, invoke one `emery` verb, and relay its output. Skill markdown must not grow orchestration, synthesis, or validation prose.

When a skill currently does something deterministic in prose (parsing YAML, validating shape, transitioning state), the right fix is to add a CLI verb and have the skill call it. The wrong fix is to make the skill smarter. See [AGENTS.md](../../AGENTS.md).

Never hand-edit `.emery/` state (the component cache, the generation store); never `mkdir -p .emery/...`. Route through the CLI — it enforces the legal set of states and validates inputs in one place for humans, agents, and CI alike.

## Verb tree

- `emery specify <adapter>... [--value <adapter>=<text>] [--sources <path>]` — the spec generator (ADR-0008 §3): resolve the sources named on the invocation (mirroring a local component into the project cache), extract, reconcile, synthesise, and commit `spec.md` / `design.md` as one generation behind the swapped `current` pointer. The binding list is per-run input, never persisted. Invoked without a source it exits `2` with `specify-source-required`; mixing `--sources` with argv bindings exits `2` with `argument`.
- `emery show <spec|design>` — print a reviewable document of the current generation; text stdout is the document body alone. Before any commit it fails `spec-not-generated` (exit `1`).
- `emery completions <shell>` — auto-derived shell completions over the live clap surface.

## JSON envelope

Every CLI verb that skills consume emits a stable **flat body**: the command-specific fields at the top level of a single JSON object. On success the body is exactly that — there is no `ok` discriminant, no `data` wrapper around the payload, and no top-level envelope-version stamp. On failure the flat object carries three top-level keys: `error` (a kebab-case discriminant string), `message` (a humanised one-liner), and `exit-code` (the integer the binary returns). Skills invoked with `--format json` parse the body and branch on the `error` field rather than on stdout text.

Stream roles are part of the contract: the semantic result body (text or JSON) is stdout; the failure body and live host tracing are stderr. In text mode the failure body's `error:` line renders in ANSI red so it stands out from the surrounding tracing; `NO_COLOR` (any non-empty value), a missing `TERM`, and `TERM=dumb` all disable it, and the JSON envelope never carries styling. Host tracing is selected by the reserved host log flags, peeled from argv before the guest sees it: bare invocations default to INFO progress, `--quiet` turns tracing off, and `--debug` adds backend debug tracing (both flags win over any ambient `RUST_LOG`). Skills follow the plugin rule's tracing contract and relay the semantic result once without repeating tracing lines.

The canonical envelope shapes live in [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md). SKILL.md bodies **link** to that reference rather than embedding envelope JSON inline (house style applied in review).

The `error` discriminants are part of the public contract that skills and tests grep for. Examples skills handle today:

- `specify-source-required` — `emery specify` without a source binding.
- `spec-not-generated` — `emery show` before any generation is committed.
- `adapter-cli-too-old` — an adapter's declared `emery` compatibility floor is newer than the running binary.

## Exit codes

The CLI uses a four-slot exit-code table. The authoritative definition (variants and the mapping from `Error::*` types) lives in [`AGENTS.md`](../../AGENTS.md#exit-codes). Summary for skills:

| Code | Name | Skills see it on |
|---|---|---|
| `0` | `EXIT_SUCCESS` | Command succeeded; parse the body. |
| `1` | `EXIT_GENERIC_FAILURE` | Default `Error` mapping; parse the top-level `error` discriminant. |
| `2` | `EXIT_VALIDATION_FAILED` | Validation errors, argument errors, clap usage errors. |
| `3` | `EXIT_VERSION_TOO_OLD` | `Error::AdapterCliTooOld` (`adapter-cli-too-old`) — an adapter's declared `emery` compatibility floor is newer than this binary; tell the operator to update the installed binary through its install channel. |

Skills should branch on the exit code first (success vs failure class) and on the top-level `error` discriminant second (the specific failure mode). New exit codes are not invented by skills or the CLI; if a class of failure does not fit the four slots, the wire contract changes in the CLI repo and the kebab `error` discriminant distinguishes the case within an existing slot.

## Cross-references

- [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md) — canonical envelope shapes per verb.
- [`AGENTS.md`](../../AGENTS.md) — authoritative source for exit codes, error variants, and CLI architecture.
