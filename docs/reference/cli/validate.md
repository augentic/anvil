# specify validate

Structural and semantic artifact validation.

## Synopsis

```bash
specify validate <change-dir>
```

## Description

Runs the validation engine against a change directory. Checks include:

- **Structural checks** -- artifact files exist, conform to expected format, required sections present.
- **Referential checks** -- specs referenced in the proposal exist, requirement IDs are unique and stable.
- **Schema checks** -- artifacts conform to the active schema's rules.

## Output

Returns a JSON validation report with three classifications:

| Classification | Meaning |
|---------------|---------|
| **Pass** | Check passed |
| **Fail** | Check failed -- must be fixed |
| **Deferred** | Check requires semantic judgment -- flagged for agent review |

The Pass/Fail/Deferred model lets the CLI handle structural checks while the agent evaluates semantic ones. See the [Decision Log](../../appendices/decision-log.md) for the rationale.
