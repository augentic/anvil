---
id: CORE-006
title: Adapter Manifest Version
severity: important
trigger: "An `adapters/{sources,targets}/<name>/adapter.yaml` declares a `version:` field whose value is not an exact `x.y.z` semver string, so the workflow loader cannot key the adapter's identity (RFC-47) off the manifest version."
rule_hints:
  - kind: path-pattern
    value: "adapters/sources/*/adapter.yaml"
    description: Source adapter manifests; `version:` must be an exact semver string per `source.schema.json`.
  - kind: path-pattern
    value: "adapters/targets/*/adapter.yaml"
    description: Target adapter manifests; `version:` must be an exact semver string per `target.schema.json`.
  - kind: regex
    value: '^version:\s*"?\d+(\.\d+)?"?\s*$'
    description: Flag a top-level `version:` line whose value is a bare integer or two-component number (e.g. `1`, `"2"`, `1.0`) rather than an exact `x.y.z` semver string. One finding per non-conforming manifest version line.
---

## Rule

Every `adapters/{sources,targets}/<name>/adapter.yaml` declares its identity through the top-level `version:` field. Per RFC-47 the adapter contract pins this field to an **exact semver string** (`x.y.z`, with optional `-prerelease` / `+build`); resolution keys on this value as the adapter's identity, synthesized target refs render `name@<semver>`, and `SourceAdapter.version` / `TargetAdapter.version` parse it into a typed `semver::Version`. A manifest that ships a bare integer (`1`, `2`) or a two-component number (`1.0`) cannot be loaded, because the loader's `adapter-version-malformed` gate rejects anything that is not exact semver.

The `path-pattern` hints scope the candidate set to adapter manifests; the `regex` hint then scans each candidate line-by-line and flags a `version:` line whose value is a bare integer or two-component number — the realistic pre-RFC-47 drift. An exact `x.y.z` value (`1.0.0`, `"1.0.0"`) does not match the forbidden pattern, so the rule fires zero findings against the current tree and surfaces only on drift. Non-numeric garbage (`version: latest`) is caught upstream by the schema (CORE-001) and the loader's `adapter-version-malformed` gate.

## Look For

- A newly added adapter manifest copy-pasted from an older scaffold that still ships `version: 1` (or any bare integer) instead of an exact semver string.
- A manifest that quoted a partial version (`version: "1.0"`) for YAML-style consistency; the contract expects all three semver components (`1.0.0`).
- A manifest scaffolded without the `version:` field at all, relying on schema-default behaviour the loader does not provide (caught by the schema, not this regex).

## Fix

Set the manifest's top-level `version:` field to an exact semver string:

```yaml
name: <adapter-name>
version: "1.0.0"
```

If the manifest genuinely needs a different version, set the full `x.y.z` value; the identity flows through the loader and the synthesized `name@<semver>` refs without any further coordination. CORE-006 is the on-disk canary; the policy authority is the per-axis JSON Schema (`source.schema.json` / `target.schema.json`) and the loader's `adapter-version-malformed` gate.
