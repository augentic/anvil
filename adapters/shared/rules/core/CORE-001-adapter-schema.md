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
    description: Validate each adapter manifest against the embedded `adapter.schema.json` shape (kebab `name`, semver-string `version`, axis discriminator, single-sentence `description`, optional `specify` floor / `inputs` / `platforms`).
---

## Rule

Every source and target adapter ships a manifest at `adapters/sources/<name>/adapter.yaml` or `adapters/targets/<name>/adapter.yaml`. The CLI loads these manifests at adapter resolve time, so a manifest that does not match the schema is a hard failure surface for the rest of the workflow. Operation sets are not declared on the wire — they derive from the closed WIT contract (`wit/specify.wit`) per axis, and each adapter's prompts are compiled into its guest.

`adapter.schema.json` pins the closed shape:

- `name` — kebab-case identifier; MUST match the directory name under `sources/` or `targets/`.
- `version` — exact semver string (`x.y.z`, with optional `-prerelease` / `+build`); the adapter's identity (RFC-47) that resolution keys on.
- `axis` — `source` or `target`. The per-axis schemas (`source.schema.json` / `target.schema.json`) lock this to a single literal; this shared shape is the common-denominator validation that runs against every manifest before the per-axis schemas refine the result.
- `description` — single-sentence human-readable summary; required.
- `specify` — optional host-CLI compatibility floor (RFC-47 D3).
- `inputs` — optional target-only build-input declarations (relative paths, `required` flag).
- `platforms` — optional target-only declarative platforms capability.

## Look For

- A new adapter directory whose `adapter.yaml` is missing one of the four required keys (`name`, `version`, `axis`, `description`).
- A retired pre-cutover key (`briefs:`, `tools:`, `extension:`) that the post-cutover closed key set rejects.
- An `axis:` value outside the closed `{source, target}` set.
- An `inputs[]` entry whose `path` is absolute or a URI.
- A `name:` that does not match the parent directory name on disk.

## Fix

Open the failing `adapter.yaml`, compare it against the schema fields listed above, and either populate the missing or malformed key or align the value with the closed enum / pattern. The schema is the canonical authority — adapter resolve and the guest-routed operations all depend on a clean manifest.
