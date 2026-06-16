# RFC-47: Adapter Identity — Semver Version and Resolve Signature

> Status: Draft · Execution order: **1st of RFC-47 → RFC-48 → RFC-49**, executed to completion in numerical sequence. The semver-identity foundation both later RFCs build on; lands first and independently. · Depends: [Adapter loader axis routing](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-loader-axis-routing), [Per-adapter versioning — forward position only](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#per-adapter-versioning--forward-position-only), the adapter loader (`crates/workflow/src/adapter/`) · Paired with: [RFC-48: Adapter packaging and transport](rfc-48-adapter-packaging-transport.md) (the distribution half — immutable fetch, content digest, shared store) · Roadmap: activates the versioning portion of [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model).

## Abstract

An adapter's identity is a **semver version**. `omnia@1.0.0` names exactly one adapter, and resolution keys on that version everywhere — the plan binding, the project's target pin, the loader probe. This RFC specifies the identity *type* (a semver string on the manifest, a `semver::Version` newtype in the loader), the resolve *signature* (a value type threading `(name, version)` to probe time), and the adapter's **host-CLI compatibility floor** (`requires-cli`) — the transport-independent guard that stops an independently-versioned adapter from loading against a CLI too old to honor it. It is the transport-independent core: how the bytes behind `omnia@1.0.0` are packaged, published, fetched, verified, and cached is [RFC-48](rfc-48-adapter-packaging-transport.md). The two split cleanly because nothing about naming an adapter by version depends on where its bytes live.

## Motivation

Today `omnia@v1` pins a *repo ref* of `augentic/specify`: the adapter's content is whatever that ref's tree carries, and `adapter.yaml.version` is a descriptive schema field, not a resolution key (see [DECISIONS.md §"Per-adapter versioning — forward position only"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#per-adapter-versioning--forward-position-only)). Two properties move identity from a repo ref to the adapter itself:

- **`version` is identity.** The manifest version is the semver string resolution keys on, matching the semver already used for tool declarations (`tool.schema.json`).
- **Identity threads through resolution as a value.** What `name` resolves to comes from the pinned `(name, version)` recorded for the project, carried through the loader by a small value type rather than a bare positional argument — so the deferred RM-21 surface (ranges, namespacing) can extend it without re-breaking every call site.

This is the smallest, highest-leverage slice of the adapter-ecosystem work: it lands the identity *type* with no distribution change, and every later transport decision keys on it.

## Principles

- **Identity is a semver, not a moving ref.** `(name, version)` names exactly one adapter. Whether that resolves to one immutable artifact is [RFC-48](rfc-48-adapter-packaging-transport.md)'s job; this RFC only fixes that the *name* is a version.
- **Resolution stays project-local in semantics.** What `name` resolves to comes from the pinned `(name, version)` recorded for the project, never "whatever is globally installed" — preserving [DECISIONS.md §"Resolution is project-local only"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-loader-axis-routing).
- **Threading is additive.** The version rides existing binding shapes as an optional field; the bare-string shorthand keeps its `None`-means-default semantics, so existing `plan.yaml` source binds parse unchanged.
- **Compatibility is declared and checked at resolve.** An adapter states the minimum CLI it needs (`requires-cli`) and the loader enforces it at probe time, transport-independently — the adapter-level analog of `project.yaml.specify_version`.
- **Pre-1.0 major cut, no migration framework.** Per [DECISIONS.md §"Bootstrap and upgrade lifecycle"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#bootstrap-and-upgrade-lifecycle), this is a major bump: re-init, not migration. No compatibility aliases for the superseded `version: 1` integer.

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Semver version** | `adapter.yaml.version` is a required semver **string** (`^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$`) that resolution keys on as identity. | `version` is a string with the semver `pattern` in `schemas/{source,target}.schema.json` and the embedded `SOURCE_JSON_SCHEMA` / `TARGET_JSON_SCHEMA`; `SourceAdapter` / `TargetAdapter` carry a `semver::Version` newtype (`adapter/core.rs`); on-disk manifests move from `version: 1` to `version: "1.0.0"`. |
| **D2 Version threaded to probe time** | Resolution carries the requested version through an `AdapterRef { name, version: Option<semver::Version> }` value type so the probe targets `<name>@<version>`. The pin lives in `plan.yaml` `sources` / `project.yaml.adapter` and carries a semver; a bare name with no version resolves the single installed identity or errors `adapter-version-required`. v1 resolves **exact pins only**; ranges (`^1.0`) deferred. | `AdapterRef` (`adapter/core.rs`) is the resolve argument: `*Adapter::resolve(&AdapterRef, project_dir)` across the ~12 call sites. `SliceSourceBinding` / `plan.schema.json` `sources` carry an optional `version` (additive — the bare-string shorthand still parses); targets reuse the `name@ref` carried by `project.yaml.adapter` (`AdapterUri`). `schemas/rules/{rule,resolved}.schema.json` accept `<name>@<semver>`. See [Resolve signature (D2)](#resolve-signature-d2). |
| **D3 Repo split** | Schema, loader, the `version` newtype, and the resolve signature live in `augentic/specify-cli`; manifest `version:` edits and brief/doc references in `augentic/specify`. | Per [AGENTS.md §"Note to the implementing agent"](https://github.com/augentic/specify-cli/blob/main/AGENTS.md), touching `SliceSourceBinding`, `plan.schema.json`, the adapter loader, or the `resolve` signature requires the cross-repo `rg` sweep in the same PR. |
| **D4 Host-CLI compatibility floor** | `adapter.yaml` carries an optional `requires-cli` semver **floor** (an exact minimum — no ranges, matching D2's exact-only posture). The loader compares it against the running binary's version at resolve time and refuses an adapter that needs a newer CLI, reusing the `project.yaml.specify_version` precedent. | Adds an optional `requires-cli` semver string to `schemas/{source,target}.schema.json` and the embedded `SOURCE_JSON_SCHEMA` / `TARGET_JSON_SCHEMA`; `SourceAdapter` / `TargetAdapter` carry `requires_cli: Option<semver::Version>` (`adapter/core.rs`); a resolve post-schema gate raises `adapter-cli-too-old` on the exit-3 `EXIT_VERSION_TOO_OLD` path (`Error::CliTooOld` family). Transport-independent — the check is identical whether bytes came from git, a registry, or a local path. See [Host-CLI compatibility floor (D4)](#host-cli-compatibility-floor-d4). |

### Version on a manifest (D1)

```yaml
name: omnia
version: "1.0.0"
axis: target
execution: agent
```

The integer `version: 1` retires with no compatibility alias — pre-1.0, a major cut means re-init, not migration. The semver-string shape matches `tool.schema.json`'s existing `tools[].version` precedent, so the schema work is a known pattern rather than a new one.

### Resolve signature (D2)

Resolution carries identity through a value type rather than a positional argument, so the deferred RM-21 surface (ranges, namespacing) can extend it without re-breaking the ~12 call sites:

```rust
// crates/workflow/src/adapter/core.rs
pub struct AdapterRef {
    pub name: String,
    pub version: Option<semver::Version>, // None = the single installed identity, else `adapter-version-required`
}

// SourceAdapter::resolve(&AdapterRef, project_dir)
// TargetAdapter::resolve(&AdapterRef, project_dir)
```

Threading is additive at every boundary:

- `SliceSourceBinding` (`change/plan/core/model.rs`) carries `version: Option<semver::Version>`; `plan.schema.json` `sources` carries an optional `version`. The bare-string shorthand keeps its `None`-means-default semantics, so existing `plan.yaml` source binds parse unchanged.
- Targets do not get a parallel field: the version source is the `name@ref` already parsed from `project.yaml.adapter` by `AdapterUri` (`init/adapter_uri.rs`).
- Because `SliceSourceBinding`, `plan.schema.json`, and the `resolve` signature all cross the two-repo contract, the [AGENTS.md §"Note to the implementing agent"](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) `rg` sweep across both repos is mandatory in the same PR (D3).

Exact pins only: a requested `version` that is `Some(_)` matches a single installed identity by equality. Range resolution (`^1.0`, `~1.2`) against a release index is RM-21 and rides this same value type when it lands.

### Host-CLI compatibility floor (D4)

Once adapters carry their own semver ([D1](#design)) and ship from a separate repo on an independent cadence ([RFC-48](rfc-48-adapter-packaging-transport.md) D7), an adapter can be built against CLI behavior — a new schema field, a brief-contract shape, a tool-permission form — that an older binary cannot honor. Without a declared floor that mismatch surfaces as a confusing parse or validation error deep in resolution, not a clear "your CLI is too old for `omnia@2.0.0`." `project.yaml.specify_version` already gives the *project* exactly this guarantee against the binary (the exit-3 `Error::CliTooOld` path); `requires-cli` gives the *adapter* the same guarantee at adapter granularity.

```yaml
name: omnia
version: "2.0.0"
requires-cli: "0.28.0"   # minimum platform / CLI version this adapter needs
axis: target
execution: agent
```

- **Exact floor, no ranges.** `requires-cli` is a single minimum the binary must meet or exceed — the same exact-only posture D2 takes on version pins. A semver-*range* compatibility policy or a cross-version matrix is RM-21 and rides this same field when it lands.
- **Checked at the loader probe.** The floor is enforced where D2's resolve signature already runs — a post-schema gate in `*Adapter::resolve` — so it is transport-independent and fires identically for `Local`, `Cached`, and registry-fetched trees. [RFC-48](rfc-48-adapter-packaging-transport.md) ships the field inside the artifact like any other manifest byte; it adds no transport machinery.
- **Reuses the exit-3 path.** A binary below the floor aborts with `adapter-cli-too-old` on the existing `EXIT_VERSION_TOO_OLD` (`Error::CliTooOld` family), so the operator gets the same "upgrade the CLI" signal `project.yaml.specify_version` already produces.
- **Optional and additive.** An absent `requires-cli` means no floor — every adapter that omits it resolves exactly as today, so the field is back-compatible at the schema boundary.

### CLI surface

No new top-level verbs. Identity flows through existing resolve paths:

```bash
specify init omnia@1.0.0            # exact semver pin recorded on project.yaml.adapter
specify source survey <source>      # resolves the bound (name, version)
specify slice build <slice>         # target resolution unchanged in shape
```

### Finding codes

| Code | Decision | Severity / kind | Raised when |
| --- | --- | --- | --- |
| `adapter-version-required` | D2 | violation (exit 2) | a bind omits version and resolution cannot pick a single installed identity for the name |
| `adapter-version-malformed` | D1 | violation (exit 2) | `version` is not valid semver (belt-and-suspenders past the schema) |
| `adapter-cli-too-old` | D4 | error (exit 3) | the running binary's version is below an adapter's declared `requires-cli` floor; reuses the `EXIT_VERSION_TOO_OLD` / `Error::CliTooOld` path |

The `adapter-digest-mismatch` finding belongs to verification, which is [RFC-48](rfc-48-adapter-packaging-transport.md).

### Test plan

- **D1** — schema parity tests for the string `version` (`crates/schema/tests/schemas.rs`); manifest round-trip tests for the `semver::Version` newtype; a malformed-version test asserting `adapter-version-malformed`.
- **D2** — a `resolve` test asserting `(name, version)` probe targeting; a bare-name-with-no-installed-identity test asserting `adapter-version-required`; a bare-`SliceSourceBinding` back-compat parse test; a `plan.yaml` round-trip test for the new optional `version`.
- **D4** — a manifest parse test for the optional `requires-cli` semver (and a malformed-floor rejection); a resolve test asserting `adapter-cli-too-old` (exit 3) when the binary is below the floor; a floor-satisfied test asserting a clean resolve; an absent-`requires-cli` back-compat test asserting no floor check.

`cargo make ci` (`RUSTFLAGS=-Dwarnings`) gates the lot.

## Phasing

1. **D1 — semver string.** Smallest, highest-leverage; has in-repo precedent (tool/marketplace/framework versions). Lands the identity *type* with no distribution change.
2. **D2 — `AdapterRef` resolve signature.** Additive at the plan/binding boundary; proceeds in parallel after D1.
3. **D4 — host-CLI compatibility floor.** An additive optional manifest field plus a resolve-time gate reusing the exit-3 `CliTooOld` path; lands after D1 (it needs the semver `version`) and alongside D2's resolve work.

All three decisions are transport-independent and can land before any of [RFC-48](rfc-48-adapter-packaging-transport.md) is scheduled — RFC-48's immutable-fetch and content-digest decisions key on the semver identity established here.

## Alternatives considered

- **Keep the integer `version: 1` and pin by repo ref.** Rejected as the forward position — a repo ref spans infinite commits, so two adapters published from one ref cannot have distinct identities, and `adapter.yaml.version` stays descriptive rather than authoritative. This is the status quo RM-21 is chartered to replace.
- **Positional `version` argument on `resolve`.** Rejected — the deferred RM-21 surface (ranges, third-party namespacing) would re-break every call site. A value type absorbs those extensions additively.
- **Semver **range** resolution now (`^1.0`, `~1.2`).** Deferred — v1 is exact pins only. The `AdapterRef` value type is the seam to widen when a release index exists.
- **Defer `requires-cli` to RM-21 with the rest of the compatibility surface.** Rejected for the *exact floor* — RFC-48 extraction + RFC-49's single platform version give adapters an independent cadence *now*, so a first-party adapter can already outrun an old CLI before any third-party ecosystem exists. The exact floor (D4) is the minimum guard that makes independent cadence safe; only *range* policy and a compatibility matrix stay deferred.

## Non-Goals

- **Packaging, publishing, fetching, content digest, and the shared store** — all [RFC-48](rfc-48-adapter-packaging-transport.md). This RFC says what an adapter is *named*; RFC-48 says how its bytes travel and how identity is *proven*.
- Semver range resolution and third-party namespacing (`org/name@req`) — RM-21, riding the `AdapterRef` value type.
- Semver-*range* `requires-cli` policy and a cross-version compatibility matrix — RM-21. The exact-floor `requires-cli` guard is **D4** (same exact-only posture as D2).
- Any migration framework — pre-1.0 this is a re-init major cut.

## References

- [DECISIONS.md §"Per-adapter versioning — forward position only"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#per-adapter-versioning--forward-position-only) — the deferred position this RFC partially activates.
- `crates/workflow/src/adapter/{core,resolve}.rs` — the loader, the `version` newtype, and the `AdapterRef` resolve signature (D1/D2).
- `crates/workflow/src/init/adapter_uri.rs` — the `project.yaml.adapter` pin parser that carries the target version.
- [AGENTS.md §"Exit codes"](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) / `src/runtime/output.rs` (`Error::CliTooOld`, `EXIT_VERSION_TOO_OLD`) and `project.yaml.specify_version` — the project-level CLI floor D4 mirrors at adapter granularity.
- `crates/workflow/src/change/plan/core/model.rs` (`SliceSourceBinding`) and `schemas/plan/plan.schema.json` — the additive optional `version` on a source bind (D2).
- `schemas/{source,target}.schema.json`, `schemas/tool.schema.json` — the manifest `version` schemas (D1) and the semver-string precedent in `tool.schema.json`.
- [RFC-48: Adapter packaging and transport](rfc-48-adapter-packaging-transport.md) — the distribution half this RFC pairs with.
- [Roadmap RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model) — the ecosystem item both RFCs serve.
