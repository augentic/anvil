# Add a Capability to an Existing Project

When your project already has a baseline at `.specify/specs/`, adding a new capability follows the standard define-build-merge loop. If you are modifying an existing capability, Specify generates delta specs automatically.

## Adding a new capability

```text
/spec:define "Add a notification service that sends emails on order completion"
```

Specify detects that `notification` does not exist in the baseline and generates a fresh spec with new `REQ-XXX` IDs.

```text
/spec:build
/spec:merge
```

After merge, `.specify/specs/notification/spec.md` appears in the baseline.

## Modifying an existing capability

```text
/spec:define "Add SMS support to the notification service"
```

Specify detects that `notification` already exists in the baseline and generates a **delta spec** with `ADDED`, `MODIFIED`, or `REMOVED` sections. Only the changes are described -- unchanged requirements are omitted.

```text
/spec:build
/spec:merge
```

The merge applies the delta to the baseline spec, preserving existing requirements and adding the new ones.

## See also

- [Iterating on a Baseline](../tutorials/iterating-on-baseline.md) -- tutorial covering delta specs in detail
- [Artifact Format](../reference/artifact-format.md) -- delta spec format reference
