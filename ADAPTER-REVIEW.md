# Pre-2.0 Architecture Review

Two-part review against the RFC-25 / Specify 2.0 merge. Pre-1.0 — no back-compat carries, so wire-format decisions taken here are effectively permanent at the 2.0 cut.

- **Part 1 — Adapter layer.** `adapter.yaml` shape, the per-axis schemas, `Adapter::resolve` loader, brief / tool / cache-fingerprint contracts. Lives in the `specify` repo plus the schemas the `specify-cli` repo ships.
- **Part 2 — CLI surface.** Public JSON envelopes, `plan.yaml` shape, journal taxonomy, exit codes, error discriminants. Lives almost entirely in the `specify-cli` repo, with downstream effects on skill bodies in `specify`.

Within each part, items are ordered by "how hard to change later" rather than by importance:

- **A** — wire-format decisions that lock at 2.0.
- **B** — internal type shape; changeable post-2.0 but cheap to do now.
- **C** — decisions worth recording in `DECISIONS.md`, no code change required.
- **D** — adjacent boundary / co-tenancy concerns.

A combined ranking and verification appendix lives at the end of the document.

---

# Part 1 — Adapter layer

Scope: the `adapter.yaml` shape, the per-axis schemas, the `Adapter::resolve` loader, and adjacent contracts (briefs, declared tools, cache fingerprinting).

## 1.A. Wire-format decisions that lock at 2.0

### 1.A1. Collapse `operations[]` into `briefs.keys()`

**Strongest recommendation.** The field is decorative — the per-axis schema already closes the operation set on the `briefs` side.

`source.schema.json` requires both `operations: [enumerate, extract]` and `briefs.{enumerate,extract}`:

```20:47:schemas/source.schema.json (specify-cli)
"operations": {
  "type": "array",
  "uniqueItems": true,
  "minItems": 2,
  "maxItems": 2,
  "items": { "type": "string", "enum": ["enumerate", "extract"] }
},
"briefs": {
  "type": "object",
  "additionalProperties": false,
  "required": ["enumerate", "extract"],
  "properties": {
    "enumerate": { … },
    "extract":   { … }
  }
}
```

Same shape in `target.schema.json` for `[shape, build, merge]`. Every required entry in `operations[]` is also a required key in `briefs.*`. There's no manifest you can author where they legally differ.

**Action**

1. Drop `operations:` from the manifest body in all 9 in-tree adapters.
2. Remove the `operations` property from `adapter.schema.json`, `source.schema.json`, `target.schema.json`.
3. Remove `pub operations: Vec<String>` from `Adapter` in `crates/domain/src/adapter/core.rs`; expose `Adapter::operations()` deriving from `briefs.keys()` if any caller needs the iterator.
4. Audit downstream call sites (`rg 'manifest\.operations\b' specify-cli/`).

**Quality delta**: −9 lines on disk · −1 schema property per file (×3) · −1 Rust field · −1 drift surface for hand edits.

**Why pre-2.0**: removing the field later is a wire break for everyone who has authored a `version: 2` adapter against the field. Schema additions are cheap; schema deletions are not.

**Counter-argument**: "The field documents the operation list at the top of the manifest." Loses because `briefs:` is two lines below and lists the same names with their paths — the documentation already lives there.

---

### 1.A2. Require `version` on `toolDeclaration`

Currently optional:

```68:81:schemas/source.schema.json (specify-cli)
"toolDeclaration": {
  "additionalProperties": false,
  "required": ["name"],
  "properties": {
    "name":    { "$ref": "#/$defs/kebabName" },
    "version": { "type": "string", "minLength": 1 },
    …
```

The in-tree state already disagrees with itself:

```yaml
# adapters/sources/code-runtime/adapter.yaml
tools:
  - name: fixture-index            # no pin
```

```yaml
# adapters/targets/contracts/adapter.yaml
tools:
  - name: contract
    version: 0.3.0                 # pinned
```

The cache fingerprint (RFC-27 §D8) incorporates declared-tool versions. Absent pins mean "this tool's version doesn't participate in the cache key" — the worst-case failure mode for reproducibility because the cache silently hits across binary changes.

**Action**

1. Move `version` into `required` on `toolDeclaration` in all three schemas.
2. Add the pin to `adapters/sources/code-runtime/adapter.yaml` (currently the only unpinned declaration).
3. Decide: semver (`0.3.0`) only, or also `sha256:<digest>` for byte-exact pins. The schema description already mentions both; lock it in `DECISIONS.md`.

**Quality delta**: +1 schema required-field · +1 manifest line · −1 reproducibility footgun.

**Counter-argument**: "Tools without a stable release shouldn't be forced to pin." Loses because the alternative is a silent-non-determinism mode — better to require a `sha256:` digest pin for unreleased tools than to allow no pin at all.

---

### 1.A3. Close the `tools[].permissions[]` grammar

```77:81:schemas/source.schema.json (specify-cli)
"permissions": {
  "type": "array",
  "uniqueItems": true,
  "items": { "type": "string", "minLength": 1 }
}
```

Free-form strings. There is no schema-side check that `read:source-dir` is a real permission name. The matching WASI runtime has a closed set; the schema should mirror it so typos fail at validate time instead of "denied" at runtime.

**Action**

1. Enumerate the runtime's permission set (likely under `wasi-tools/` or `crates/tool/`).
2. Replace the `items` clause with either an `enum: ["read:source-dir", …]` or a `pattern: "^(read|write):[a-z-]+$"` constraint.

**Quality delta**: −1 silent-typo failure mode. Widening the enum later is non-breaking.

**Risk**: low — additive constraint on a field that's already a closed set in the runtime.

---

### 1.A4. Require `description` on the manifest

Today optional:

```47:49:schemas/source.schema.json (specify-cli)
"description": { "type": "string", "minLength": 1 }
```

But every in-tree adapter sets it. Making it required formalises the in-practice contract and gives `specify source list` / `specify target list` (current or future) a guaranteed one-liner per adapter.

**Action**

1. Move `description` into the `required` array on `adapter.schema.json`, `source.schema.json`, `target.schema.json`.
2. (No manifest edits — every in-tree adapter already conforms.)

**Quality delta**: +1 schema required-field · +0 manifest lines · −1 "shipping an unlabelled adapter" failure mode.

**Counter-argument**: "Tiny adapters don't need a description." Loses because the cost is one sentence and the benefit is operator-facing surface area.

---

## 1.B. Internal type shape (changeable post-2.0, but cheap to do now)

### 1.B1. Type the operations into the Rust manifest

`crates/domain/src/adapter/operation.rs` already carries the closed target enum:

```19:31:specify-cli/crates/domain/src/adapter/operation.rs
pub enum Operation {
    Shape,
    Build,
    Merge,
}
```

`cache.rs` carries the closed source enum (`SourceOperation { Enumerate, Extract }`). Yet the manifest itself stores raw strings:

```95:97:specify-cli/crates/domain/src/adapter/core.rs
pub operations: Vec<String>,
pub briefs: BTreeMap<String, String>,
```

…and `brief_path` takes `&str`:

```302:304:specify-cli/crates/domain/src/adapter/core.rs
pub fn brief_path(&self, root_dir: &Path, operation: &str) -> Option<PathBuf> {
    self.briefs.get(operation).map(|relative| root_dir.join(relative))
}
```

**Action**

1. Split `Adapter` into `SourceAdapter` / `TargetAdapter` (or one generic `Adapter<Op>`).
2. `briefs: BTreeMap<SourceOperation, String>` / `BTreeMap<TargetOperation, String>`.
3. `brief_path(operation: TargetOperation)` etc. — push the string boundary out to the YAML parse step.
4. Combine with 1.A1: once `operations[]` is gone, `briefs.keys()` is the canonical operation iterator.

**Quality delta**: −2 string-keyed lookups (move to compile-time) · +1 generic parameter or +1 type · YAML wire unchanged.

**Why pre-2.0 is the moment**: not a wire issue, but the cleanup is half the work if it lands alongside 1.A1.

---

## 1.C. Worth a `DECISIONS.md` entry, not a schema change

### 1.C1. Name uniqueness across axes

Nothing today prevents `adapters/sources/omnia/` and `adapters/targets/omnia/` from coexisting. The resolver always takes an axis, so it works; cache directories are partitioned (`.specify/.cache/adapters/sources/omnia/` vs `…/targets/omnia/`). But operator wire output (journal events, validation errors) sometimes refers to "adapter `omnia`" without the axis label.

**Decision options**

- Pin "names are unique across axes" — enforce at `specify init` and `Adapter::resolve`.
- Pin "names are per-axis; output always carries the axis label" — current behaviour, document it.

Either works; both are forever-decisions. My default is option 2 (matches existing behaviour).

---

### 1.C2. Reserve an optional `requires.cli` field

`project.yaml.specify_version` already gates project ↔ CLI. Adapters don't have an equivalent. If a future feature adds a schema field the CLI must understand (see 1.C3), the only escalation today is `version: u32` major bump.

**Action**

Reserve `requires: { cli: ">=2.0" }` in `adapter.schema.json` now (`additionalProperties: false` keeps the door open without an enforcer). The field can ship unused at 2.0; the cost is one schema property and one paragraph in `DECISIONS.md`.

**Alternative**: don't reserve, and accept one-cycle of "must bump major to express requirement" the first time it's needed.

---

### 1.C3. Brief I/O contract (deferred)

Briefs reference `$SOURCE_DIR` as a convention enforced only by human authoring. Eventually a declarative I/O block per operation buys lint / dry-run / env-var checks without parsing markdown:

```yaml
briefs:
  extract:
    path: briefs/extract.md
    reads:  [$SOURCE_DIR]
    writes: [$SLICE_DIR/evidence/<source-key>.yaml]
```

**Do not land in 2.0.** Surface is large; enforcer isn't there. But record the shape in `DECISIONS.md` so future proposals argue against this baseline rather than re-litigating from scratch.

---

### 1.C4. Deprecation mechanism

Today there's no way to say "`code-typescript` was renamed to `code-ts` in 2.x". An optional `deprecated: { since, replacement, sunset }` block costs nothing reserved in the schema and saves a hard rename cut later.

**Action**

Same flavour as 1.C2 — reserve the field name in `adapter.schema.json`, no enforcer at 2.0. Documented in `DECISIONS.md`.

---

## 1.D. Adjacent: `plugin.json` vs `adapter.yaml` boundary

`plugins/spec/.cursor-plugin/plugin.json` and `adapter.yaml` share *zero* discovery surface — Cursor and `specify` are different runtimes and that's correct. But the repo has no doc explaining the relationship, and "is there a JSON config for adapters?" is a known recurring question.

**Action**

One short paragraph in `AGENTS.md` (or a new `docs/explanation/adapter-anatomy.md` ↔ `docs/explanation/plugin-anatomy.md` pair) covering:

- Cursor plugin manifests (`.cursor-plugin/plugin.json`) register Cursor IDE surface area (skills, rules, slash commands).
- Adapters (`adapter.yaml`) are loaded by the `specify` CLI via `Adapter::resolve(axis, name, project_dir)`.
- The two manifest systems are independent; they share no fields and no loader.

Prevents the same question recurring post-2.0.

---

## Part 1 — Findings dropped before publication

| Candidate | Why dropped |
|---|---|
| Drop the redundant `axis:` field (directory already encodes it) | Defense-in-depth wins — `axis:` travels with the manifest if it's ever published to a registry detached from its directory. |
| Add a per-adapter `defaults:` / `config:` block | `plan.yaml` slice bindings already carry slice-scoped config; manifest-level overrides duplicate that surface. Out of scope for 2.0. |
| Bump `version: u32` to semver | Major-only is sufficient given the closed contract; semver inflates surface without a consumer. |
| Add front matter to briefs (YAML header on each markdown file) | Strict contract gain, but the markdown body is the source of truth — front matter duplicates what 1.C3 would express via the manifest, more cleanly. |
| Force `tools[].permissions[]` non-empty | Empty list ≡ no grants, which is a legal posture; no need to require at least one. |

---

# Part 2 — CLI surface

Scope: public JSON envelopes (`ErrorBody`, per-command `*Body`), `plan.yaml` shape, the journal taxonomy, exit codes, kebab error discriminants, and the schemas/README that documents them. Most contracts already live in `specify-cli/DECISIONS.md` — these are the holes.

## 2.A. Wire-format decisions that lock at 2.0

### 2.A1. `Plan::sources` is the 1.x bare-string form — schema says otherwise

**Most concrete pre-2.0 risk.** The schema permits both 1.x and RFC-25 shapes:

```69:101:specify-cli/schemas/plan/plan.schema.json
"sourceBinding": {
  "oneOf": [
    {
      "type": "string",
      "minLength": 1,
      "description": "1.x backward-compat: bare string carries a path or URL; the adapter is inferred by the caller."
    },
    {
      "type": "object",
      "additionalProperties": false,
      "required": ["adapter"],
      "properties": {
        "adapter": { "$ref": "#/$defs/kebabName", … },
        "path":    { "type": "string", … },
        "value":   { "type": "string", … }
      }
    }
  ]
}
```

But the Rust type is bare-string only, with an explicit deferred-TODO comment:

```97:105:specify-cli/crates/domain/src/change/plan/core/model.rs
/// The on-disk shape is currently a bare-string value per key
/// (1.x backward-compat). RFC-25 widens this to a structured
/// `{ adapter, path?, value? }` object — that loader change is
/// W0.3's responsibility, not W0.2's.
#[serde(default)]
pub sources: BTreeMap<String, String>,
```

…and the CLI flattens to bare:

```20:34:specify-cli/src/commands/plan/create.rs
pub fn build_source_map(sources: Vec<SourceArg>) -> Result<BTreeMap<String, String>> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    for SourceArg { key, value } in sources {
        …
        map.insert(key, value);
    }
    Ok(map)
}
```

Today's state: **the schema documents an interface the loader cannot read**. AGENTS.md is explicit that "2.0 is a hard cut from 1.x — no compatibility aliases." Land one of two resolutions before merge:

- **Land the structured loader (the deferred W0.3 task).** Change `Plan::sources` to `BTreeMap<String, SourceBinding>` where `SourceBinding` mirrors the `SliceSourceBinding::{Bare, Structured}` pattern already in `model.rs:254`. The CLI gains `--source key=adapter:path` (or similar) to author the structured shape.
- **Drop the structured branch from the schema.** Keep `sourceBinding` as bare-string only. Defer the structured form to a future RFC. The schema is then honest about what the loader accepts.

Either way the on-disk wire format is decided forever at 2.0. The status quo (schema permissive, loader strict) is the worst of both worlds: a structured `plan.yaml` validates clean and then crashes on parse.

**Why pre-2.0**: post-2.0, choosing either branch requires either a wire break or a schema break, depending on which way operators have been authoring.

---

### 2.A2. The closed journal taxonomy has no "negative outcome" events

`EventKind` is documented as closed (DECISIONS.md §"Journal event names"). The current taxonomy covers transitions and synthesis tags but never the failure cases the lifecycle model explicitly tolerates:

```16:19:specify-cli/crates/domain/src/change/plan/core/model.rs
/// `done` (written by `plan transition <name> done` — the final per-entry
/// transition, stamped by `/spec:merge`). Build failures and merge conflicts
/// leave the active entry `in-progress`; v1 has no per-entry `blocked`,
/// `failed`, or `skipped` state.
```

A slice can park `in-progress` for hours or days after a build failure or merge conflict, and there is **no journal event** for "this slice failed to build" or "merge conflict reached". Observers tailing `journal.jsonl` see `slice.transition.refined` and then silence until something resolves. The same is true for archive — `plan.transition.archived` doesn't exist; the file simply moves.

Adding a journal event post-2.0 is *technically* additive (consumers should ignore unknown event ids), but the DECISIONS.md table is labelled as the closed taxonomy and skill bodies / tests filter on it.

**Action** — decide one of:

- Add the missing transitions to the closed set now: `slice.build.failed`, `slice.merge.conflicted`, `plan.transition.archived`. Even if the emitter sites aren't wired yet, defining the wire shape is the forever decision. Cost: ~30 LOC of `EventKind` variant + one row per event in the docs table.
- Reclassify the taxonomy from "closed" to "open with kebab discriminants", so future additions don't need to be flagged as breaking. Cost: one paragraph in DECISIONS.md.

**Why pre-2.0**: regret cost. Adding `slice.build.failed` post-2.0 means every skill body that filters journal events grows a new branch and every test that asserts on the closed set updates.

---

### 2.A3. `target: "omnia@v1"` versioning — parsed or decorative?

`plan.yaml.slices[].target` is declared as a free string:

```148:151:specify-cli/schemas/plan/plan.schema.json
"target": {
  "type": "string",
  "description": "RFC-25 target-adapter identifier (e.g. `omnia`, `contracts@v1`). …"
}
```

In-repo test fixtures use the versioned form:

```200:200:specify-cli/crates/domain/src/change/plan/core/validate/tests.rs
adapter: "omnia@v1".to_string(),
```

But `adapter.yaml` carries `version: u32` (`version: 1`, integer not `"v1"`). Three things to decide:

- Is `@vN` parsed at all? `rg '@v[0-9]'` against the loader will answer.
- If parsed, does it have to agree with the resolved adapter's `version` field?
- If decorative, why does every test fixture carry it?

**Action**

The forever-decision is the wire shape: do operators write `omnia` or `omnia@v1` in `plan.yaml`? Pick one. Pre-2.0 is when "free string accepting both" stops being acceptable. Document the policy in `DECISIONS.md` alongside the existing `Divergence` and `SliceSourceBinding` entries.

---

### 2.A4. Schema directory naming drift

`schemas/README.md` lists the parent shared shape as `plugin.schema.json`:

```9:12:specify-cli/schemas/README.md
| Schema | Purpose |
|---|---|
| [`adapter.schema.json`](adapter.schema.json) | Validates a Specify adapter manifest (`adapter.yaml`) per RFC-13 §Adapter manifest and protocol. (Pre-RFC-25; retained for v1.x manifests until the W0.3 loader replacement.) |
| [`plugin.schema.json`](plugin.schema.json) | Shared shape for RFC-25 source and target adapter manifests — the axis-discriminated parent schema. |
```

But `plugin.schema.json` doesn't exist on disk — DECISIONS.md confirms it was renamed to `adapter.schema.json` in the F9 collapse. The README never got updated, and the entry for the *current* `adapter.schema.json` still describes the pre-RFC-25 v1.x shape. `tool.schema.json` exists on disk but isn't listed in the README at all.

**Action**

1. Rewrite `schemas/README.md` to reflect the post-F9 state — `adapter.schema.json` is the shared shape; `plugin.schema.json` is gone.
2. Add the missing `tool.schema.json` row.
3. Drop the "RFC-13 §Adapter manifest" reference; replace with "RFC-25 §Adapter implementation shape".

This is doc rot, but it's *the* doc consumers point at to learn the wire surface. Fixing it pre-2.0 makes the schemas/ tree the official "schemas at 2.0" artefact.

---

## 2.B. Internal type shape (no wire impact)

### 2.B1. Schema/Rust cross-field rules for `target` × `project`

The model says:

```122:127:specify-cli/crates/domain/src/change/plan/core/model.rs
/// Target-adapter identifier (RFC-25 §Adapter vocabulary) for the
/// slice (e.g. `omnia@v1`, `contracts@v1`). Required when
/// `project` is `None`; optional override when `project` is
/// `Some`. Mutually enriching with `project`: …
```

JSON Schema can express this with a `oneOf` / `anyOf` cross-field constraint, but `plan.schema.json` currently makes both `target` and `project` optional with no cross-field rule. The Rust validator owns the constraint.

**Action** — decide one of:

- Move the rule into the schema (cross-field `oneOf` keyed on `project: null` vs string). Gives external schema consumers (IDEs, CI tools that don't link the Rust validator) the same rule.
- Document explicitly in `DECISIONS.md` that the schema is shape-only and `Plan::validate` owns cross-field rules.

Whichever — the decision is "where does cross-field validation live."

---

### 2.B2. `Exit` enum has 5 variants for 4 slots

```10:23:specify-cli/src/output.rs
pub enum Exit {
    Success,
    GenericFailure,
    ValidationFailed,
    VersionTooOld,
    /// Argument-shape failure …
    ArgumentError,
    /// WASI tool exit-code passthrough …
    Code(u8),
}
```

DECISIONS.md's table lists 4 slots; the Rust enum has 5 named variants (plus `Code(u8)`). `Exit::ArgumentError` and `Exit::ValidationFailed` both map to code 2 but are distinct variants. The exit code is the wire contract, so this is fine on the wire — but the documentation says "four-slot table" when the type has five named slots colliding into four codes.

**Action**

Add a row to the `DECISIONS.md` exit-code table: `Exit::ArgumentError → 2`, alongside the existing `Exit::ValidationFailed → 2` entry. Two lines of doc edit; closes a "is there an `ArgumentError` code I should map?" question.

---

## 2.C. Worth a `DECISIONS.md` entry, not a code change

### 2.C1. `Status::Done` is absorbing

There's no way to un-`done` a slice. If a slice merged, the upstream PR was reverted, and the operator wants to redo the work, the only escape hatch is hand-editing `plan.yaml`. This is probably intentional (RFC-25 says merge is the final per-entry transition) but it's not in `DECISIONS.md`.

**Action**

One paragraph in `DECISIONS.md` confirming `Done` is absorbing in v1. Saves the inevitable "why isn't there a `plan transition --undo` verb?" thread post-2.0.

---

### 2.C2. Archive lives outside the lifecycle

`plan transition <plan> reviewed` is the only plan-level transition. There's no `archived` state — archive moves the file to `.specify/archive/` and the lifecycle stamp stays `reviewed`.

**Action**

One paragraph in `DECISIONS.md` ("archive is a filesystem operation, not a lifecycle state") so it doesn't get re-litigated. Tied to 2.A2 if `plan.transition.archived` lands as a journal event without a corresponding lifecycle state.

---

### 2.C3. The `@vN` target suffix policy (tied to 2.A3)

If 2.A3 lands as "decorative", document it. If it lands as "parsed and reconciled", document it. Either way `DECISIONS.md` should pin which adapter targets parse the suffix; everyone copy-pasting test fixtures should know whether `omnia@v1` is required, optional, or wrong.

---

### 2.C4. Source-key namespace collision policy

`plan.yaml.sources.<key>` keys and `plan.yaml.slices[].sources[].key` reference the same kebab namespace; the schema permits two slices to bind the same key to different candidates because the per-slice binding includes the candidate id.

**Action**

One sentence in `DECISIONS.md`: "source keys are plan-scoped; each key maps to exactly one binding under `Plan::sources`, but slices may reference the same key with different candidates." Codifies how the code already behaves and removes a "should I namespace source keys per-slice?" question.

---

## 2.D. Adjacent: cache co-tenancy

`Adapter::locate` already defends against this:

```276:283:specify-cli/crates/domain/src/adapter/core.rs
// The cache root co-tenants with RFC-27 §D8's per-adapter
// result cache (`<adapter>/<fingerprint>/…` and `index.jsonl`),
// so a bare directory does not imply the manifest itself is
// cached. Probe for `adapter.yaml` instead — it is the only
// file `cache_adapter` writes at this layer.
```

Two distinct caches living in the same tree, distinguished only by a probe for `adapter.yaml`. Working today, but the comment is a sign that someone is going to add a third cache at the same level and the probe heuristic will need to grow.

**Action** — decide one of:

- Formally separate them — `.specify/.cache/manifests/{sources,targets}/<name>/` for manifests, `.specify/.cache/extractions/<adapter>/<fp>/` for fingerprinted results.
- Acknowledge the co-tenancy is deliberate, with a sentence in `DECISIONS.md` §"Tool architecture" or §"Cache layout".

Not a wire change to the journal/plan layer, so it can land post-2.0 if you don't want to commit. Just acknowledge the design intent.

---

## Part 2 — Findings dropped before publication

| Candidate | Why dropped |
|---|---|
| Add top-level `envelope-version` to JSON output | DECISIONS.md §"Wire compatibility" already pins "no envelope-version unless a breaking shape change ships." Adding one without that trigger is overhead. |
| Split `specify-domain` crate (largest non-test crate) | DECISIONS.md §"Crate layout" already pins "new functionality lands in an existing module by default." Splitting reverses the 2026-05 13→4 collapse without an enforcer reason. |
| Unify `is_kebab` predicate with `kebabName.pattern` regex | Internal invariant, not a wire shape. The Rust comment cross-references the schema; drift is detectable at test time. |
| Refactor `src/commands/plan/create.rs` (1024 LOC) into a sub-module | "New modules" forbidden per parent REVIEW.md's published rule. |
| Promote `Diag` codes to typed variants en masse | DECISIONS.md §"Diag-first error policy" already pins the inverse direction — `Diag` is the default; promotion happens on demand. |
| Add `--undo` / `plan transition --rollback` | Out of scope for 2.0; 2.C1 acknowledges `Done` as absorbing instead. |
| Force `slices[].depends-on` to topologically validate at schema time | Cross-field check beyond JSON Schema's reach; lives correctly in `Plan::validate`. |

---

# Combined merge ranking

| Tier | Items | Effort | Permanent? |
|---|---|---|---|
| Must land before 2.0 | **1.A1, 2.A1** | 1.A1 low · 2.A1 medium | yes (wire) |
| Strong before 2.0 | **1.A2, 1.A4, 2.A2, 2.A3** | low–medium each | yes (wire) |
| Opportunistic before 2.0 | **1.A3, 1.B1, 2.A4, 2.B2** | low each | mixed |
| Document before 2.0 | **1.C1–1.C4, 1.D, 2.B1, 2.C1–2.C4, 2.D** | one paragraph each | docs |
| Defer to 2.1+ | **1.C3** (brief I/O contract) | n/a | n/a |

**If you only do two things: 1.A1 + 2.A1.** Both close out half-finished wire shapes that 2.0 would otherwise freeze in their inconsistent state.

**If you do five: add 1.A2 + 1.A4 + 2.A2.** Bakes reproducibility (tool-version pin), authoring clarity (description required), and observability (failure-path journal events) into the 2.0 contract.

**If you do eight: add 1.A3 + 2.A3 + 2.A4.** Closes the WASI permission grammar, the `@vN` target-suffix policy, and the schemas/README doc rot — all wire-locking decisions, all cheap.

Everything in C and D is "write one paragraph in DECISIONS.md" — no code change, no schema change. The `specify-cli/DECISIONS.md` you've already written is genuinely thorough; these are small holes in coverage rather than new ground.

---

# Verification

For each landed item:

```bash
cd specify-cli && cargo make ci    # schemas embed via include_str!, so schema edits compile-check immediately
cd specify    && make checks       # doc + workflow consistency
```

For 1.A1 specifically, after the field removal:

```bash
rg -n '^operations:' adapters/
# (no matches)
rg -n 'manifest\.operations\b|\.operations\.iter' specify-cli/src specify-cli/crates
# (only the operations() accessor remains)
```

For 1.A2 specifically:

```bash
rg -nA2 '^tools:' adapters/
# every block under `tools:` shows a `version:` line
```

For 2.A1 specifically (if landing the structured loader):

```bash
rg -n 'pub sources: BTreeMap<String, String>' specify-cli/crates/domain
# (no matches — replaced by SourceBinding)
rg -n '"adapter":' specify-cli/tests/fixtures
# structured form appears in at least one fixture
```

For 2.A1 (if landing the schema rewind instead):

```bash
rg -n '"sourceBinding"' specify-cli/schemas/plan/plan.schema.json
# field gone or `type: string` only
```

For 2.A2:

```bash
rg -n 'SliceBuildFailed|SliceMergeConflicted|PlanTransitionArchived' specify-cli/crates/domain/src/journal.rs
# (variants present if option 1; absent + DECISIONS.md paragraph if option 2)
```

For 2.A4:

```bash
rg -n 'plugin\.schema\.json|RFC-13' specify-cli/schemas/README.md
# (no matches)
rg -n 'tool\.schema\.json' specify-cli/schemas/README.md
# (one match, in the table)
```
