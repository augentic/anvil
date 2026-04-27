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
- **Composition checks** (Vectis only) -- structural validation of `composition.yaml`: valid YAML, `version: 1`, exactly one of `screens` or `delta` (not both), kebab-case screen slugs. Cross-artifact checks: `maps_to` values are non-empty strings, field coverage (every view struct field has a `bind`), event coverage (every shell-facing Event has an `event` wiring), ViewModel mapping, overlay trigger consistency, and navigation graph consistency. See [Artifact Format > Composition](../artifact-format.md#composition-document-vectis-only) for the full checklist.

## Output

Returns a JSON validation report with three classifications:

| Classification | Meaning |
|---------------|---------|
| **Pass** | Check passed |
| **Fail** | Check failed -- must be fixed |
| **Deferred** | Check requires semantic judgment -- flagged for agent review |

The Pass/Fail/Deferred model lets the CLI handle structural checks while the agent evaluates semantic ones. See the [Decision Log](../../explanation/decision-log.md) for the rationale.

## See also

- [/spec:define](../change-skills/define.md) -- skill that invokes validation
- [Artifact Format](../artifact-format.md) -- expected artifact structure
- [Decision Log](../../explanation/decision-log.md) -- rationale for Pass/Fail/Deferred
