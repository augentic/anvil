# Technical Design

Document the technical approach for implementing the spec changes.

## Instructions

1. Read the approved proposal and spec deltas
2. Read the current-state analysis
3. Document:

### Domain Model Changes
- New types, modified types, removed types
- Field additions/removals/renames with exact Rust types
- Serde attribute changes
- Entity relationship changes (use mermaid ER diagram)

### API Contract Changes
- New endpoints: method, path, request/response shapes
- Modified endpoints: what changed in the contract
- Removed endpoints
- For each: authentication requirements, error responses

### Provider Capability Changes
- New provider traits needed and why
- Removed provider traits (if capabilities are being removed)
- New config keys, removed config keys

### Business Logic
- Algorithm changes per handler (pseudocode with tags)
- New validation rules
- Changed error handling

### Structural Changes
- Type renames (old name → new name, all affected files)
- Handler splits or merges
- Module reorganization

### Guest Wiring Changes
- New routes/topics to add
- Routes/topics to remove
- Import changes

Use mermaid diagrams for entity relationships and sequence flows.