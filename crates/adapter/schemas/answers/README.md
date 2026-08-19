# Judgment answer schemas

Each document is the `format: schema(...)` payload an adapter guest sends with a judgment `omnia:model/completion.create` call. They are committed here, owned by the adapter SDK beside the seam wire types that deserialise the answers (`emery_adapter::answers`), and pinned by `crates/adapter/tests/answers.rs`. Never edit them by hand — a wire-type change regenerates the schema in the same change.

| Schema                 | Answer for                              |
| ---------------------- | ---------------------------------------- |
| `evidence.schema.json` | source `extract` — the extracted claim set |
