Create the design document to explain HOW to implement the change.

Create full design if any of the following apply:
  - Cross-cutting change (multiple services/modules) or new
    architectural pattern
  - New external dependency or significant data model changes
  - Security, performance, or migration complexity
  - Ambiguity that benefits from technical decisions before coding

If none of the above apply, create a minimal design.md noting that a full
design is not warranted and referencing the proposal and specs:

Structure the document with crate-specific sections if multiple crates are involved.

Required sections (see template):
  - **Context**: Source, purpose, background, and current state
  - **Domain Model**: Entity and type definitions with field names,
    types, wire names, and optionality
  - **API Contracts**: Endpoints with method, path, request/response
    shapes, and error responses
  - **External Services**: Name, type (API, table store, cache, message
    broker), authentication method
  - **Constants & Configuration**: All config keys with descriptions
    and defaults
  - **Business Logic**: Per-handler tagged pseudocode using [domain],
    [infrastructure], [mechanical] tags. Include required provider
    traits per handler.
  - **Publication & Timing Patterns**: Topics, message shapes, timing
  - **Implementation Constraints**: Platform or runtime constraints
  - **Source Capabilities Summary**: Checklist of required capabilities
  - **Dependencies**: External packages or services
  - **Risks / Open Questions**: Known risks and unresolved decisions
  - **Notes**: Additional observations

Focus on the technical shape needed for implementation. Reference the
proposal for motivation and specs for behavioral requirements. Use
mermaid diagrams for entity relationships and flows.
