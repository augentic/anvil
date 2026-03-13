---
name: epic-analyzer
description: Produce Specify artifacts (proposal.md, specs/, design.md) from JIRA epic and user stories via MCP server.
argument-hint: [epic-key] [change-dir]
allowed-tools: Read, Write, StrReplace, Shell, mcp__atlassian__*
---

# Epic Analyzer Skill

## Overview

Fetch a JIRA epic and its linked user stories via the JIRA MCP server, then produce Specify artifacts (proposal.md, specs/, design.md) capturing requirements, acceptance criteria, and BDD scenarios. Domain model, API contracts, and business logic are written directly to `design.md` from extracted JIRA data (technical documentation sections, attachments, and requirements-inferred content).

## Derived Arguments

1. **Epic Key** (`$EPIC_KEY`): JIRA epic key (e.g., "ATR-7102")
2. **Change Directory** (`$CHANGE_DIR`): Output directory for Specify artifacts (e.g., `./.specify/changes/my-crate`)

```text
$EPIC_KEY      = $ARGUMENTS[0]
$CHANGE_DIR    = $ARGUMENTS[1]
$CRATE_NAME    = basename($CHANGE_DIR)
$SPECS_DIR     = $CHANGE_DIR/specs
$DESIGN_PATH   = $CHANGE_DIR/design.md
$PROPOSAL_PATH = $CHANGE_DIR/proposal.md
```

## Principles (Non-Negotiable)

1. **Comprehensive Extraction**: Capture all requirements, acceptance criteria, and BDD scenarios
2. **Direct Design Generation**: Write `design.md` directly from extracted JIRA data — technical documentation sections, attachments, and requirements-inferred content
3. **Zero Invention**: Do not invent requirements or functionality not present in JIRA
4. **Explicit Unknowns**: Use `[unknown]` tags for ambiguous or missing information
5. **Traceability**: Each requirement must be traceable to a JIRA issue
6. **Tagging**: Tag business logic as `[domain]`, `[infrastructure]`, `[mechanical]`, or `[unknown]`

## Tags and Unknown Tokens

See complete definitions in [Specify Artifact Format - Tags Reference](references/specify.md#tags-reference) and [Unknown Tokens Reference](references/specify.md#unknown-tokens-reference).

## MCP Tool Usage

### Discovering Available Tools

The MCP project directory is at `$HOME/.cursor/projects/<project-slug>/mcps/`. List its contents to discover available servers, then list the `jira/tools/` subdirectory for available JIRA tools.

```bash
# List MCP servers (adjust the project slug to match this workspace)
ls ~/.cursor/projects/*/mcps/

# If jira/ folder exists, list available tools
ls ~/.cursor/projects/*/mcps/jira/tools/
```

### Expected JIRA MCP Tools

Based on `@modelcontextprotocol/server-jira`, expected tools include:

- `get_issue` — Fetch single issue by key
- `search_issues` — Search issues with JQL
- `get_issue_comments` — Fetch comments for an issue
- `get_epic_issues` — Fetch all issues in an epic

### Example MCP Call

```text
CallMcpTool:
  server: "jira"
  toolName: "get_issue"
  arguments: {"issueKey": "ATR-7102"}
```

## Process

### Step 1: Verify MCP Server Configuration

1. List available MCP servers to check if JIRA MCP server is configured
2. If not configured, provide setup instructions and exit with error
3. List available JIRA MCP tools to understand capabilities

### Step 2: Fetch JIRA Epic

1. Use MCP tool to fetch epic by key (`$EPIC_KEY`)
2. Extract epic metadata:
   - Key, Summary, Description, Status, Labels, Components
   - Priority, Assignee, Reporter, Created, Updated
3. Extract custom fields if present (e.g., Acceptance Criteria)

### Step 3: Fetch Linked User Stories

1. Search for issues linked to the epic (via epic link or parent relationship)
2. For each user story, extract:
   - Key, Summary, Description, Status
   - Acceptance Criteria (from Description or custom field)
   - BDD Scenarios (look in Description, Comments, or "Other Information" section)
   - Story Points, Priority
3. Preserve hierarchical relationships

### Step 3.5: Extract Technical Documentation from Descriptions

Parse epic and user story descriptions for structured technical documentation sections:

1. **Identify Markdown Sections**:
   - Look for headers like `## Technical Specifications`, `## API Contract`, `## Domain Model`, `## External Services`, `## Database Schema`, `## Configuration`
   - Extract content under these headers until next same-level header

2. **Supported Section Types**:
   - `## Technical Specifications` — General technical details
   - `## API Contract` / `## API Contracts` — Endpoint specifications
   - `## Domain Model` — Entity definitions
   - `## External Services` / `## External Dependencies` — External integrations
   - `## Database Schema` — Database structure
   - `## Configuration` — Required environment variables and config

3. **Content Preservation**:
   - Preserve JSON code blocks as-is
   - Preserve YAML code blocks as-is
   - Preserve SQL code blocks as-is
   - Preserve tables and lists
   - Maintain formatting and structure

4. **Source Tracking**:
   - Tag each extracted section with source: Epic or User Story key

### Step 3.6: Fetch Issue Attachments (Optional)

Attempt to fetch attachments from epic and user stories via MCP:

1. **Check MCP Capability**:
   - Try to call MCP tool for attachments (e.g., `jira_get_attachments`, `jira_download_attachments`)
   - If tool not available, skip gracefully and log in design.md Notes

2. **For Each Issue** (Epic + User Stories):
   - Call MCP tool to list attachments
   - Filter for relevant file types: `.json`, `.yaml`, `.yml`, `.md`, `.sql`, `.txt`

3. **Download and Parse**:
   - For JSON/YAML: Parse as structured data
   - For Markdown: Extract as formatted text
   - For SQL: Extract as code block
   - For text files: Extract raw content

4. **Error Handling**:
   - If MCP tool not available: Log "Attachment support not available" in design.md Notes
   - If attachment download fails: Log failure with filename
   - Continue processing remaining attachments

5. **Size Limits**:
   - Skip attachments larger than 1MB
   - Log skipped files in design.md Notes

### Step 4: Parse Acceptance Criteria

For each user story, parse acceptance criteria:

1. **Natural Language Format**:

   ```text
   - User can perform action X
   - System validates condition Y
   - Error message Z is displayed when...
   ```

2. **Structured Format**:

   ```text
   Given: preconditions
   When: action
   Then: expected outcome
   ```

3. Extract as structured requirements list

### Step 5: Parse BDD Scenarios

Parse BDD scenarios from "Other Information", Comments, or Description:

```gherkin
Scenario: Description
  Given precondition
  And another precondition
  When action occurs
  Then expected result
  And another expected result
```

Extract:

- Scenario name
- Given steps (preconditions)
- When steps (actions)
- Then steps (assertions)

### Step 6: Write Proposal and Specs

Create the `$CHANGE_DIR/` directory structure and write the proposal and spec files.

#### 6a. Write proposal.md

Write `$PROPOSAL_PATH` from the JIRA epic data:

```markdown
## Why

<Problem or opportunity from the JIRA epic description>

## What Changes

- <List of capabilities being added/modified, derived from user stories>

## Capabilities

### New Capabilities

- `capability-name`: one-line description (one per user story or logical capability)

## Impact

- Affected services, APIs, dependencies (from technical documentation in Steps 3.5-3.6)
```

#### 6b. Write spec files

Write a single consolidated spec file at `$SPECS_DIR/$CRATE_NAME/spec.md`
using the flat baseline format, following the [Deriving Specs From
JIRA](references/specify.md#deriving-specs-from-jira-epic-analyzer) format:

```markdown
# <Capability Name> Specification

## Purpose

<1-2 sentence description from user story summary>

### Requirement: <Behavior Name>

ID: REQ-001

The system SHALL <behavioral description from acceptance criterion>.
Source: JIRA $STORY_KEY, Criterion $N

#### Scenario: <Happy Path>

- Given: <preconditions>
- When: <trigger/input>
- Then: <expected output/behavior>

#### Scenario: <Error Case>

- Given: <preconditions>
- When: <invalid input or error condition>
- Then: <error response with expected status/code>

## Error Conditions

- <error type>: <description, HTTP status, and conditions>
```

Each spec contains:

- **Purpose** — from user story summary
- **Requirements** — from acceptance criteria (each criterion becomes a top-level requirement with a stable `ID: REQ-XXX` line and Given/When/Then scenarios)
- **Error Conditions** — from acceptance criteria and BDD scenarios describing error paths
- **Source traceability** — every requirement linked to its JIRA issue key

**Do NOT include in specs**: Algorithm pseudocode, domain model details, API contract shapes — these belong in `design.md`.

### Step 7: Write design.md

Write `$DESIGN_PATH` directly from the JIRA data gathered in Steps 1-5, following the [Design Document Format](references/specify.md#design-document-format).

**Content sources** (in priority order — use structured sections verbatim when available, infer from requirements text when not):

1. **Context** — From epic summary and description
2. **Domain Model** — From `## Domain Model` sections extracted in Step 3.5 (verbatim), or inferred from nouns/attributes in requirements text. Mark inferred content with `<!-- synthesized from requirements text -->`.
3. **API Contracts** — From `## API Contract` sections extracted in Step 3.5 (verbatim), or inferred from acceptance criteria verbs and preconditions/outcomes. Mark inferred content with `<!-- synthesized from requirements text -->`.
4. **Business Logic** — From structured business rules in Step 3.5, validation rules and algorithmic steps from acceptance criteria and BDD scenarios. Tag each rule: `[domain]`, `[infrastructure]`, `[mechanical]`, or `[unknown]`.
5. **External Services** — From Step 3.5 `## External Services` / `## External Dependencies` sections, or `[unknown — not specified in JIRA]` if absent
6. **Constants & Configuration** — From Step 3.5 `## Configuration` sections, or `[unknown — not specified in JIRA]` if absent
7. **Publication & Timing Patterns** — From event/messaging sections in JIRA data, or omit if absent
8. **Source Capabilities Summary** — Inferred from which generic capabilities the component requires (Configuration, Outbound HTTP, Message publishing, Key-value state, Authentication/Identity, Table/database access, Real-time messaging)
9. **Implementation Constraints** — From technical requirements and architecture notes

**Validation**:

- [ ] `$DESIGN_PATH` exists and is valid
- [ ] Domain Model section present with entities and attributes
- [ ] API Contracts section present with endpoints
- [ ] Business Logic section present with tagged algorithm steps
- [ ] External Services section present

### Step 8: Verify Final Artifacts

Read back the artifacts in `$CHANGE_DIR/` and verify:

1. `$PROPOSAL_PATH` exists with Why, What Changes, Capabilities, and Impact sections
2. Spec file exists at `$SPECS_DIR/$CRATE_NAME/spec.md` with flat `### Requirement:` blocks and stable `ID: REQ-XXX` lines
3. Each spec has Purpose, Requirements with Given/When/Then scenarios, and Source traceability to JIRA
4. `$DESIGN_PATH` exists with Domain Model, API Contracts, Business Logic, and External Services
5. All artifacts follow the format specified in [specify.md](references/specify.md)
6. Requirements artifacts guidance passes (see [Validation Checklists](references/specify.md#validation-checklists))

If any required sections are missing, add an `[unknown]` token in the relevant artifact noting the gap.

## Reference Documentation

The Specify artifact format and downstream consumers are documented in shared references:

- **[Specify Artifact Format](references/specify.md)** -- Artifact structure with spec file templates, design document format, and proposal format
- **[Examples](references/examples/)** -- Complete worked examples showing JIRA data to Specify artifact conversion

For more detail on how the code-analyzer skill produces artifacts from source code, see the `code-analyzer` skill.

## Examples

Detailed examples are available in the `references/examples/` directory:

1. [simple-epic.md](references/examples/simple-epic.md) - Analyze a simple epic with 2-3 user stories
2. [epic-with-bdd.md](references/examples/epic-with-bdd.md) - Epic with comprehensive BDD scenarios in "Other Information"
3. [epic-with-technical-docs.md](references/examples/epic-with-technical-docs.md) - Epic with technical documentation in markdown sections and attachments

Each example includes:

- JIRA epic and user story keys
- Expected JIRA data structure
- MCP tool calls used
- Generated Specify artifact excerpts (proposal.md, specs/, design.md)
- Technical documentation extraction (where applicable)

See the examples directory for complete details.

## Error Handling

### MCP Server Not Configured

**Symptom**: MCP server folder not found or empty

**Resolution**:

1. Display setup instructions from Prerequisites section
2. Exit with error: "JIRA MCP server not configured. See setup instructions above."

### Epic Not Found

**Symptom**: MCP returns 404 or "Issue not found"

**Resolution**:

1. Verify epic key is correct
2. Check JIRA_URL and authentication in MCP config
3. Ensure user has access to the epic

### Missing Custom Fields

**Symptom**: Acceptance Criteria or BDD scenarios not found

**Resolution**:

1. Check Description field for acceptance criteria
2. Look in Comments for BDD scenarios
3. Mark as `[unknown — not specified in JIRA]` if truly missing

### JQL Search Failures

**Symptom**: Cannot find user stories linked to epic

**Resolution**:

1. Try alternative search: `"Epic Link" = $EPIC_KEY`
2. Try: `parent = $EPIC_KEY`
3. If both fail, document in design.md Notes section

## Verification Checklist

Before completing, verify:

- [ ] MCP server is available and configured
- [ ] Epic fetched successfully with all metadata
- [ ] All linked user stories retrieved
- [ ] Acceptance criteria extracted from each story
- [ ] BDD scenarios parsed with Given/When/Then structure
- [ ] Technical documentation sections extracted from descriptions
- [ ] Attachments fetched (if MCP supports it)
- [ ] `$PROPOSAL_PATH` written with Why, What Changes, Capabilities, Impact
- [ ] Spec file written to `$SPECS_DIR/$CRATE_NAME/spec.md` with flat `### Requirement:` blocks, stable `ID: REQ-XXX` lines, Requirements, and BDD Scenarios
- [ ] Each spec has Source traceability to JIRA issue keys
- [ ] `$DESIGN_PATH` written with Domain Model, API Contracts, Business Logic, External Services
- [ ] All unknowns explicitly marked
- [ ] Artifacts follow format specified in [specify.md](references/specify.md)

## Important Notes

- Do not invent requirements not present in JIRA
- Preserve exact wording from acceptance criteria
- Use `[unknown]` liberally when requirements are ambiguous
- Traceability is critical: link each item to source JIRA issue
- Focus on reconstruction-grade accuracy for downstream code generation
- **Direct design generation**: Epic-analyzer writes `design.md` directly from gathered JIRA data. Structured technical documentation sections from descriptions (Step 3.5) and attachments (Step 3.6) are included verbatim. Content inferred from requirements text is marked with `<!-- synthesized from requirements text -->` for transparency.
- **Artifact format**: All output follows the [Specify Artifact Format](references/specify.md). Specs capture behavioral "what" (requirements, scenarios); design.md captures technical "how" (domain model, algorithms); proposal.md captures "why" (motivation, scope).
