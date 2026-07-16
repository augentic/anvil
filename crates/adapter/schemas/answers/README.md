# Vendored judgment answer schemas

These documents are **vendored copies** of the generated answer schemas published by [augentic/specify](https://github.com/augentic/specify). Each is the `format: schema(...)` payload an adapter guest sends with a judgment `omnia:model/completion.create` call. They are never hand-written: upstream generates each one (via `schemars`) from the Rust wire type that deserialises the answer — generation lives in `project::answers` (`crates/project/src/answers.rs`), and the committed upstream goldens sit at `crates/project/answers/`, parity-gated by `crates/project/tests/answers.rs`:

| Schema                 | Answer for                                                                                 |
| ---------------------- | ------------------------------------------------------------------------------------------ |
| `leads.schema.json`    | source `survey` — `{ leads: [...] }`                                                       |
| `evidence.schema.json` | source `extract` — Evidence minus the envelope `lead`                                      |
| `report.schema.json`   | target `build` / `merge` — report minus `version` / `slice` / `target`, diagnostic inlined |

This copy is a temporary pin: once the `specify:adapter` package distribution carries the answer schemas (see [`wit/README.md`](../../../../wit/README.md)), this directory is deleted. Until then, keep it byte-identical to the upstream goldens under `crates/project/answers/`. Never edit these files here; change the Rust wire type in specify, regenerate with `REGENERATE_GOLDENS=1`, and re-vendor.
