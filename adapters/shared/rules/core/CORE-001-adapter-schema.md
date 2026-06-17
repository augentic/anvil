---
id: CORE-001
title: Adapter Manifest Schema
severity: critical
trigger: An `adapter.yaml` manifest under `adapters/**/` fails to validate against the shared adapter manifest schema (missing required fields, unknown keys, or malformed values).
rule_hints:
  - kind: path-pattern
    value: adapters/**/adapter.yaml
    description: Narrow the candidate set to adapter manifests before schema validation.
  - kind: schema
    value: adapter
    description: Validate each adapter manifest against the embedded `adapter.schema.json` shape (kebab `name`, semver-string `version`, axis discriminator, `briefs` map, optional `tools` declarations, single-sentence `description`).
---

## Rule

Every source and target adapter ships a manifest at `adapters/sources/<name>/adapter.yaml` or `adapters/targets/<name>/adapter.yaml`. The CLI loads these manifests at adapter resolve time and routes every plan- and slice-time operation through the briefs they declare, so a manifest that does not match the schema is a hard failure surface for the rest of the workflow.

`adapter.schema.json` pins the closed shape:

- `name` — kebab-case identifier; MUST match the directory name under `sources/` or `targets/`.
- `version` — exact semver string (`x.y.z`, with optional `-prerelease` / `+build`); the adapter's identity (RFC-47) that resolution keys on.
- `axis` — `source` or `target`. The per-axis schemas (`source.schema.json` / `target.schema.json`) lock this to a single literal and close the legal `briefs.keys()` set; this shared shape is the common-denominator validation that runs against every manifest before the per-axis schemas refine the result.
- `description` — single-sentence human-readable summary; required.
- `briefs` — map from operation name to a relative brief path; absolute paths and URIs are rejected.
- `extension` — optional singular WASI extension declaration (`name?` run handle + `permissions`); a per-extension `version` / `source` / `sha256` is rejected (the wasm rides the adapter's own semver identity and content digest).

## Look For

- A new adapter directory whose `adapter.yaml` is missing one of the five required keys (`name`, `version`, `axis`, `description`, `briefs`).
- A `briefs:` entry whose value is an absolute path, a URL, or otherwise empty.
- An `axis:` value outside the closed `{source, target}` set, or a manifest that mixes operations from both axes (e.g. a `source` manifest that also declares `shape`).
- An `extension:` block carrying a rejected `version` / `source` / `sha256` key, or a kebab-case violation in its `name`.
- A `name:` that does not match the parent directory name on disk.

## Fix

Open the failing `adapter.yaml`, compare it against the schema fields listed above, and either populate the missing or malformed key or align the value with the closed enum / pattern. The schema is the canonical authority — adapter resolve, plan survey, and slice extraction all depend on a clean manifest.
