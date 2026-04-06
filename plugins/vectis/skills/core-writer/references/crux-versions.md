# Crux Dependency Versions

Current crates.io versions for all Crux projects. When generating or updating
`Cargo.toml` files, use these versions. Treat this file as the primary
reference for dependency versions, and keep any examples elsewhere aligned
with it.

## Workspace Dependencies

| Crate | Version | When to include |
|-------|---------|-----------------|
| `crux_core` | 0.17.0 | Always |
| `crux_http` | 0.16.0 | HTTP requests needed |
| `crux_kv` | 0.11.0 | Key-value storage needed |
| `crux_time` | 0.15.0 | Time/scheduling needed |
| `crux_platform` | 0.8.0 | Platform detection needed |

## Companion Dependencies

| Crate | Version | Notes |
|-------|---------|-------|
| `facet` | =0.31 | Exact pin required; other versions may be incompatible with `crux_core` |
| `uniffi` | =0.29.4 | Exact pin required; must match `uniffi_bindgen` bundled in `crux_core::cli` |
| `serde` | 1.0 | Always |
