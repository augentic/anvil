# RM-03 Codex RFC Review

> Purpose: review findings and resolution recommendations for `rfcs/rm-03-codex.md`.
> Scope: evaluation against three criteria — alignment with agent-centric coding-standards best practices, simplicity for initial use, and extensibility for future refinements.
> Status: advisory; intended to be folded into `rm-03-codex.md` (or addressed inline) before implementation begins.

## Summary

`rfcs/rm-03-codex.md` is in good shape and lands the major design decisions correctly:

- Markdown + YAML frontmatter as the rule format.
- Capability-distributed codex with a foundational `default` capability that always loads.
- Stable namespaced rule IDs (`UNI-`, `RUST-`, `IFACE-`, `SEC-`, `OMNIA-`, `VECTIS-`, `ORG-`).
- Citation form `<provenance>:<id>` with capability version (`default@1:UNI-002`).
- Deterministic vs model-assisted vs hybrid taxonomy for review routing.
- Staged migration that preserves `UNI-001` through `UNI-021` and wires no review until the format settles.
- Clear scope boundaries: RM-04 owns the finding schema, RM-11 owns the reviewer, suppression and waivers are explicitly deferred.

The remaining work is small. The RFC alludes to fields and behaviours it never pins down (optional frontmatter, the deterministic/model classification field, body heading rules), leaks one piece of out-of-scope work into the V1 resolver (shared catalogs), and misses a few cheap extensibility reservations whose absence will force breaking changes on the first refinement (rule lifecycle, schema version, JSON export shape).

Address the seven items in [Findings — V1 blockers](#findings-—-v1-blockers) and the RFC is ready to drive implementation. The polish items in [Findings — non-blocking](#findings-—-non-blocking) can fold in during the migration commits.

---

## Strengths

- **Format choice.** Markdown + YAML frontmatter is the dominant agent-friendly pattern (Cursor `.mdc`, Anthropic Skills, ESLint/Clippy ecosystems for citation style). Frontmatter as machine contract, body as model/human guidance is the correct split.
- **Capability-distributed storage.** Re-using the existing capability vehicle instead of inventing a parallel `codex/` root keeps surface area small and enforces "rules travel with the thing they describe." Modelling `default` as a capability is a clever framing.
- **Stable IDs and reserved namespaces.** `UNI-` / `RUST-` / `IFACE-` / `SEC-` / `OMNIA-` / `VECTIS-` / `ORG-` covers the practical space without over-reserving.
- **Citation form.** `default@1:UNI-002` embeds provenance and capability version cleanly — future-proof for RM-22 (capability ecosystem operating model).
- **Finding/rule split.** `finding_id` (occurrence) vs `rule_id` (stable) cleanly separates RM-03's responsibility from RM-04's. Avoids the trap the current `SEC-1` reviewer report style would create.
- **Deterministic vs model-assisted vs hybrid.** Realistic taxonomy. Hybrid for `SEC-*` is exactly right because regex + intent check is how those rules actually get enforced.
- **Migration sequencing.** Format validator → preserve `UNI-*` IDs → add capability codices → expose JSON → wire reviewer is the lazy-work order; nothing is wired before the format settles.
- **Restraint.** Suppression syntax, waiver workflow, dashboards, and review wiring are explicitly deferred. The Recommended Roadmap Answer at the end is a faithful summary suitable for the upstream roadmap entry.

---

## Findings — V1 blockers

These are the items I would resolve before implementation starts. They are all small.

### F1. Optional frontmatter is referenced but never enumerated

**Where:** §Rule Format, table of frontmatter fields.

**Problem:** The section says "Fields that route review, narrow applicability, or enrich catalogue browsing are optional and defaulted." The table then lists only the four required fields. Elsewhere the doc references:

- "applicability metadata" (Roadmap quote in §Context).
- "Metadata that lets `specify review` decide whether a rule is deterministic, model-assisted, or hybrid" (§Scope, In Scope).
- "filtering metadata" (Roadmap quote).

A reader cannot tell what the JSON export will actually carry beyond `id` / `title` / `severity` / `trigger`, which is awkward given the export is the contract for RM-04 and skills.

**Recommendation:** Reserve a small set of optional fields with defaults — even if V1 ignores some.

```markdown
mode: hybrid              # deterministic | model | hybrid (default: model)
status: active            # active | deprecated | experimental (default: active)
applies_to:               # all optional, all OR-combined
  capabilities: [omnia]
  languages:    [rust]
  paths:        ["src/**/*.rs"]
tags: [security, async]
```

If V1 only honours `id` / `title` / `severity` / `trigger` / `mode`, that is acceptable — but reserving the other names now prevents a breaking change on the first refinement.

### F2. The body's "required headings" are asserted but never specified

**Where:** §Migration Plan step 1 vs §Rule Format.

**Problem:** Migration step 1 says the validator should check "frontmatter shape, rule IDs, required headings, duplicate IDs, and provenance." The format section never lists which headings are required. The example uses `## Rule`, `## Look For`, `## Good`, `## Bad`, `## Spec Guidance` — mandating all of those would be heavy.

**Recommendation:** Pick one posture and state it.

- Free-form body, no required headings (recommended for "as simple as possible") — and drop "required headings" from the validator.
- Or: exactly `## Rule` is required; the rest are recommended-but-optional.

Either is defensible; both being implied is not.

### F3. Severity remap from the seed catalogue is asserted but not specified

**Where:** §Rule Format severity row + §Migration Plan step 2.

**Problem:** The seed file uses `Critical` / `Warning` / `Info`. The new model is `critical` / `important` / `suggestion` / `optional`. The doc says RM-03 "should not use the old `warning` / `info` labels as canonical" but never gives the per-label mapping. Migration is asserted to preserve IDs but is silent on whether each `Warning` becomes `important` or `suggestion`.

**Recommendation:** Add one-line guidance ("seed `Warning` → `important` by default; reviewers may downgrade per-rule during migration; seed `Info` → `suggestion`") and let the migration commit log the per-rule decisions.

### F4. The `default` capability collides with `capability.schema.json`

**Where:** §Design Summary tree + §Capability Integration.

**Problem:** The doc proposes `capabilities/default/capability.yaml` with a `codex/` directory, but the current `capabilities/capability.schema.json` is `additionalProperties: false` and requires `pipeline.{define,build,merge}` each with `minItems: 1`. A pure-codex `default` capability has no slice loop. Three resolutions are possible:

- Allow codex-only capabilities in the schema (cleanest; needs schema edit).
- Give `default` a stub pipeline (cosmetic ugliness; round-trips through the loop machinery for nothing).
- Special-case `default` in the resolver (asymmetric; harder to extend later).

The RFC waves at this in §Capability Integration ("a later capability manifest revision may add an explicit `codex:` field") but does not reckon with the codex-only case for `default` itself, which is required in V1.

**Recommendation:** Lift this to a concrete decision in §Migration. Cleanest path: a small schema relaxation that makes `pipeline` optional when the capability ships only codex, gated either by the absence of `pipeline` entirely or by an explicit `kind: codex-only | full` discriminator.

### F5. Distribution path for `<specify-distribution>` is unspecified

**Where:** §Storage And Resolution → Source Locations.

**Problem:** `<specify-distribution>/capabilities/default/codex/` is the entry point for `UNI-*`, but the doc never says where that resolves on a user's machine. Bundled in the binary? `SPECIFY_HOME`? Cached under `~/.specify/`? Discovered relative to the executable?

This is the difference between V1 working out of the box and V1 requiring manual setup. RM-02 has precedent with `.specify/context.lock`.

**Recommendation:** Add a short paragraph or call this out as a new Open Question. Recommended posture: bundle inside the `specify-cli` distribution, discoverable via a path resolved relative to the binary, with a `SPECIFY_HOME` override for development. Codex content is small enough to embed without bloating the binary.

### F6. Shared catalogs leak into the V1 resolution union

**Where:** §Storage And Resolution.

**Problem:** The resolved-set union lists shared catalogs at #3. §Shared Catalogs then says they "are optional" and "can be resolved from project config once the project config has a catalog field." Open Question #1 asks where the config lives. They are scoped-in and scoped-out in different paragraphs.

**Recommendation:** Cut shared catalogs from the V1 resolver entirely. Keep one short subsection saying the resolver design leaves room for them. Do not list them in the resolution union or in Source Locations. They re-enter cleanly in a follow-up RFC once the project config story is decided.

### F7. JSON export shape is sketched, not specified

**Where:** §Resolution Command + Open Question #4.

**Problem:** The doc says the JSON export "should include" five things but does not pin the shape. Open Question #4 asks "What is the minimum JSON export shape RM-04 needs to avoid schema churn?" — but RM-04 depends on RM-03 here, so deferring forces churn.

**Recommendation:** Pin a minimal V1 shape now, marked as `schema_version: 1`. Even an explicitly experimental shape beats deferring entirely.

```json
{
  "schema_version": 1,
  "rules": [
    {
      "id": "UNI-002",
      "title": "Unvalidated Input",
      "severity": "critical",
      "trigger": "External or user-supplied data enters code without boundary validation.",
      "mode": "hybrid",
      "status": "active",
      "body_markdown": "## Rule\n...",
      "source_path": "capabilities/default/codex/input-validation.md",
      "provenance": { "kind": "capability", "capability": "default", "version": 1 }
    }
  ]
}
```

That is the smallest shape that lets RM-04 and skills cite without retro-fitting.

---

## Findings — non-blocking

Polish that should fold in during the migration commits but does not block V1.

### F8. Mushy provenance-citation rule

**Where:** §Design Summary, "Rules are cited with both ID and provenance when provenance matters."

**Recommendation:** Replace with: "Findings always cite `<provenance>:<id>`. Human-readable CLI output may abbreviate to `<id>` when the resolved set is unambiguous."

### F9. `specify codex validate` vs `specify check` boundary

**Where:** Open Question #2.

**Recommendation:** Commit now. `specify codex validate` always exists and validates codex format only; `specify check` (RM-07) calls into it for framework-repo lint coverage. This matches the roadmap principle of keeping enforcement surfaces distinct.

### F10. `--capability <name>` filter on list/export

**Where:** §Resolution Command CLI surface.

**Recommendation:** Add `--capability <name>` to `specify codex list` and `specify codex export` from day one. Near-zero cost; makes capability-distributed storage usable immediately.

### F11. Cross-rule references in body prose

**Where:** §Rule Format.

**Problem:** `UNI-019` already references `UNI-002` in the seed text.

**Recommendation:** Note that bare IDs in body prose are permitted plain text (no special markup yet). Leaves room for later linkifying without committing to a syntax now.

### F12. Layered overrides extensibility note

**Where:** §Design Summary, "If a repo wants stricter policy than a capability rule, it should add a new local rule ID instead of redefining the capability rule."

**Problem:** This is the right V1 posture, but the wording closes a door that ESLint and Clippy both ended up needing — severity-only overrides without redefining the rule body.

**Recommendation:** Add one sentence acknowledging that a future revision may add layered severity-only overrides; V1 still rejects body redefinitions.

### F13. Body self-containment

**Where:** §Rule Format.

**Recommendation:** State explicitly that rule bodies must be self-contained — no required external link to understand the rule. Agents may not have web access; codex must be portable.

### F14. Detection metadata location

**Where:** §Hybrid Rules.

**Problem:** "Deterministic regex scanners" are mentioned but the doc never says where their config will live (frontmatter? sibling YAML? `tools.yaml`?).

**Recommendation:** One-line reservation: "A future revision will add an optional `detect:` block in frontmatter for deterministic scanner config (regex, AST query, declared tool reference). V1 ignores any `detect:` field if present."

### F15. Per-file `schema_version`

**Where:** §Rule Format.

**Recommendation:** Either reserve a top-level `schema_version: 1` frontmatter field (cheap insurance) or state explicitly that V1 is implicit-v1 and document the rule for V2 introducing the field. Implicit-without-statement is a silent foot-gun.

### F16. Rule lifecycle / `status` field

**Where:** Open Question #3.

**Recommendation:** Reserve `status: active | deprecated | experimental` in frontmatter. V1 honours `active`; `deprecated` rules stay parseable but are hidden from `codex list` by default and surfaced via `--include-deprecated`. Resolves Open Question #3 at near-zero cost.

### F17. Severity collision risk in resolved union

**Where:** §Design Summary.

**Problem:** Globally unique IDs across the resolved set with rejection on duplicate is the right V1 posture, but the doc does not say *which* CLI step performs the rejection.

**Recommendation:** State that `specify codex validate` (and the in-process resolver used by `list` / `show` / `export`) is the rejection point.

---

## Verdict against the three review criteria

### Best practices for agent-centric coding standards

**Largely yes.** The Markdown + frontmatter + stable ID + namespace model is canonical. The deterministic / model / hybrid distinction is unusually mature for a V1 design and is the right one for agent-driven review. Two specific best-practice misses, both addressable:

- Classify rule lifecycle (`status`) up front (F16).
- Pin the JSON export shape so downstream consumers do not re-litigate it (F7).

### Simplicity for initial use

**Nearly.** The four-field required frontmatter, single resolver, and "reject duplicates rather than override" rules are all the right call. The simplifications still worth making:

- Drop shared catalogs from the V1 resolver (F6).
- Trim or pin the body's "required headings" claim (F2).
- Decide the `default`-capability schema question now rather than leaving it implicit (F4).
- Pin the distribution path for the bundled default codex (F5).

### Extensibility for future refinement

**Mostly, with reservations.** Capability-distributed storage and namespace reservation are extension-friendly. The doc is missing a few cheap reservations that future revisions will want, all named in [Findings — V1 blockers](#findings-—-v1-blockers) and [Findings — non-blocking](#findings-—-non-blocking):

- Optional frontmatter field names (F1: `mode`, `status`, `applies_to`, `tags`).
- Per-file `schema_version` (F15).
- Placeholder for `detect:` metadata (F14).
- Explicit V1 JSON export shape (F7).

None of these requires V1 implementation; all should be named so V2 is not a breaking change.

---

## Recommended next steps

1. Fold F1–F7 into `rm-03-codex.md` (or address inline) before opening implementation tickets.
2. Decide the `default`-capability schema question (F4) and, if it lands as a schema relaxation, sequence the schema edit ahead of the codex validator work in §Migration step 1.
3. Pin the V1 JSON export shape (F7) and treat it as the contract that RM-04 consumes; mark it `schema_version: 1` and explicitly experimental if needed.
4. Resolve Open Questions #1–#4 by reference to F6, F9, F16, and F7 respectively. Add a new Open Question (or inline answer) covering F5 distribution path.
5. Treat F8–F17 as polish — fold in during the migration commits rather than blocking the first implementation PR.
