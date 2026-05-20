---
id: IFACE-004
title: SemVer Contract Versioning
severity: important
trigger: An OpenAPI or AsyncAPI contract is created or changed and its info.version is missing, non-SemVer, unchanged, or inconsistent with the compatibility impact.
---

## Rule

OpenAPI and AsyncAPI `info.version` values must parse as SemVer and should communicate the compatibility impact of the contract delta. Use a major version bump for breaking wire-shape changes, a minor version bump for backward-compatible additions, and a patch version bump for metadata, examples, descriptions, or other non-behavioral corrections. Do not use dates, build timestamps, branch names, or opaque release labels in place of SemVer.

Version numbers do not make a breaking change safe. They are a review and rollout signal that must agree with the consumer-impact classification and migration plan.

## Look For

- `info.version` values such as dates, free-form strings, or unquoted numbers that do not parse as SemVer.
- Breaking changes with no version bump, or with only a minor or patch bump.
- Additive changes that unnecessarily reset identity or create a new top-level contract instead of a SemVer minor bump.
- Patch bumps attached to wire-shape changes that affect payload validation, operations, channels, status codes, or message definitions.
- Multiple top-level contracts changed for one interface where versions disagree about the same compatibility impact.

## Spec Guidance

When a slice changes a public contract, require the spec or alignment report to state the intended SemVer impact. For unpublished drafts, prerelease labels are acceptable when they still parse as SemVer, such as `1.0.0-draft.1`.
