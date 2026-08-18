# Vendored judgment answer schemas

These documents are **symlinks** to the generated answer schemas in [`crates/project/answers/`](../../../project/answers/). Each is the `format: schema(...)` payload an adapter guest sends with a judgment `omnia:model/completion.create` call. They are never hand-written: upstream generates each one (via `schemars`) from the Rust wire type that deserialises the answer — generation lives in `project::answers` (`crates/project/src/answers.rs`), and the committed goldens sit at `crates/project/answers/`, parity-gated by `crates/project/tests/answers.rs`:

| Schema                 | Answer for                              |
| ---------------------- | ---------------------------------------- |
| `evidence.schema.json` | source `extract` — the extracted claim set |

This pin is temporary: once the `emery:adapter` package distribution carries the answer schemas (see [`wit/README.md`](../../../../wit/README.md)), this directory is deleted. Until then, regenerate goldens with `REGENERATE_GOLDENS=1` under `crates/project` — the symlinks pick up the new bytes automatically. Never edit schema files under this directory.
