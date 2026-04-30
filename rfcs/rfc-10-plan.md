# RFC-10 Implementation Plan

> **Source RFC**: [`archive/rfc-10-skills.md`](./archive/rfc-10-skills.md)
> **Audience**: An execution agent landing the RFC in a sequence of small, reviewable changes. Each chunk is sized so it can be handled by a single subagent without exceeding context.
> **Repository scope**: Almost all changes land in `specify/` (this repo). One narrow chunk reaches into `specify-cli/` (sibling at `/Users/andrewweston/rust/github.com/augentic/specify-cli`).

## How to use this plan

1. Work through chunks in numeric order. Cross-chunk dependencies are stated explicitly.
2. Each chunk has **scope**, **inputs**, **edits**, **acceptance**, and **dependencies**. Read the linked RFC section before starting, then return to the chunk to execute.
3. After every chunk, run `make checks` from the repo root and fix any regression in the same chunk before declaring it done. The plan is sequenced so `make checks` passes after every chunk.
4. Do **not** touch persisted artifact identifiers. The following are intentionally stable per RFC §C.3 / §E and must not change at any point during this work:
   - schema id `contracts@v1`, schema dir `schemas/contracts/`, brief id `contracts`
   - baseline dir `.specify/contracts/`, change-local dir `.specify/changes/<name>/contracts/`, subdirs `contracts/http/`, `contracts/messages/`, `contracts/schemas/`
   - validation rule ids `contracts.*` and `rules_for("contracts")`
   - registry roles `contracts.produces`, `contracts.consumes`, `contracts.imports`
5. The phrase "skill `name:` field" always means the YAML frontmatter `name:` key inside SKILL.md, not the directory name. The skill **directory** name does not change in chunks 1–4 and 14–17; it changes only in chunks 5–7 and 9–13 where the RFC explicitly renames or splits a skill.

## Inventory snapshot (pre-RFC-10)

29 SKILL.md files across six plugins:

```text
plugins/spec/skills/{init,define,build,merge,drop,extract,analyze,plan,execute}
plugins/omnia/skills/{crate-writer,test-writer,guest-writer,code-reviewer}
plugins/vectis/skills/{core-writer,core-reviewer,ios-writer,ios-reviewer,android-writer,android-reviewer,design-system-writer,test-writer,template-updater}
plugins/contracts/skills/{writer,validator,importer}        # replaced by interfaces in chunks 7-11
plugins/rt/skills/{wiretapper,replay-writer,git-cloner}     # git-cloner deleted in chunk 5
plugins/plan/skills/{sow-writer}                            # plugin renamed to client in chunk 6
```

Manifest: `.cursor-plugin/marketplace.json`. Plugin manifests: `plugins/<plugin>/.cursor-plugin/plugin.json`. Skill schema: `schemas/skill.schema.json`. Repo lints: `scripts/checks.ts` (run via `make checks`).

---

## Phase 1 — Mechanical frontmatter sweeps

These four chunks are pure find-and-replace. No structural changes, no file creation/deletion. Each chunk leaves `make checks` green.

### Chunk 1 — Drop `license: MIT` from every SKILL.md

**RFC**: §A.4

**Scope**: Every SKILL.md file under `plugins/*/skills/*/SKILL.md` (29 files, including the to-be-deleted `rt/git-cloner` and the to-be-replaced `contracts/{writer,validator,importer}`).

**Edits**:

- For each SKILL.md, delete the single line `license: MIT` from the YAML frontmatter (between the `---` delimiters).
- Do not touch any other field.
- Do not touch `plugins/<plugin>/.cursor-plugin/plugin.json` — the license declaration belongs there and stays.

**Acceptance**:

- `rg -n "^license:" plugins/**/SKILL.md` returns no matches.
- `make checks` passes (the existing schema marks `license` optional, so removal is safe).

**Dependencies**: None.

---

### Chunk 2 — Drop `allowed-tools` from every SKILL.md

**RFC**: §A.5 (policy 1: omit everywhere)

**Scope**: Every SKILL.md that currently declares `allowed-tools:`. Pre-audit: `analyze`, `extract`, `init`, `code-reviewer`, `crate-writer`, `guest-writer`, `test-writer` (omnia), `core-writer`, `core-reviewer`, `ios-writer`, `ios-reviewer`, `android-writer`, `android-reviewer`, `design-system-writer`, `test-writer` (vectis), `template-updater`, `writer` (contracts), `validator` (contracts), `importer` (contracts), `git-cloner`, `replay-writer`, `wiretapper`, `sow-writer`.

**Edits**:

- For each affected SKILL.md, delete the single line `allowed-tools: ...` from the frontmatter.
- Do not touch any other field.

**Acceptance**:

- `rg -n "^allowed-tools:" plugins/**/SKILL.md` returns no matches.
- `make checks` passes (the field is optional in the schema and has its own optional check that becomes a no-op when absent).

**Dependencies**: None. Can run in parallel with Chunk 1.

---

### Chunk 3 — Rewrite `argument-hint` per RFC §A.3 table

**RFC**: §A.3

**Scope**: Every SKILL.md whose `argument-hint` value contains `?`, `--`, `|`, or any flag-shaped token. Use this table verbatim.

| File | New `argument-hint` value |
|---|---|
| `plugins/spec/skills/init/SKILL.md` | `"[schema-url]"` |
| `plugins/spec/skills/define/SKILL.md` | `"[description]"` |
| `plugins/spec/skills/build/SKILL.md` | `"[change-name]"` |
| `plugins/spec/skills/merge/SKILL.md` | `"[change-name]"` |
| `plugins/spec/skills/drop/SKILL.md` | `"[change-name]"` |
| `plugins/spec/skills/extract/SKILL.md` | `"<source-path> <change-dir>"` |
| `plugins/spec/skills/analyze/SKILL.md` | `"<input-path> <output-dir>"` |
| `plugins/spec/skills/plan/SKILL.md` | `"<initiative-name>"` |
| `plugins/spec/skills/execute/SKILL.md` | (delete the line entirely; flag-only invocation) |
| `plugins/omnia/skills/code-reviewer/SKILL.md` | `"[crate-path]"` |
| `plugins/omnia/skills/crate-writer/SKILL.md` | `"[crate-name]"` |
| `plugins/omnia/skills/test-writer/SKILL.md` | `"[crate-name]"` |
| `plugins/omnia/skills/guest-writer/SKILL.md` | (no current value; leave omitted or add `"[crate-name]"` only if the existing body documents one) |
| `plugins/vectis/skills/test-writer/SKILL.md` | `"[feature-name]"` |
| `plugins/vectis/skills/core-writer/SKILL.md` | `"<change-dir>"` |
| `plugins/vectis/skills/ios-writer/SKILL.md` | `"<change-dir>"` |
| `plugins/vectis/skills/android-writer/SKILL.md` | `"<change-dir>"` |
| `plugins/vectis/skills/design-system-writer/SKILL.md` | `"<change-dir>"` |
| `plugins/vectis/skills/core-reviewer/SKILL.md` | `"<target-dir>"` |
| `plugins/vectis/skills/ios-reviewer/SKILL.md` | `"<target-dir>"` |
| `plugins/vectis/skills/android-reviewer/SKILL.md` | `"<target-dir>"` |
| `plugins/vectis/skills/template-updater/SKILL.md` | `"[cli-repo-dir]"` |
| `plugins/plan/skills/sow-writer/SKILL.md` | `"<change-dir>"` |
| `plugins/rt/skills/wiretapper/SKILL.md` | `"<legacy-dir>"` |
| `plugins/rt/skills/replay-writer/SKILL.md` | `"<crate-name>"` |
| `plugins/rt/skills/git-cloner/SKILL.md` | (skip — file is deleted in Chunk 5) |
| `plugins/contracts/skills/writer/SKILL.md` | (skip — replaced in Chunks 8–11) |
| `plugins/contracts/skills/validator/SKILL.md` | (skip — replaced in Chunks 8–11) |
| `plugins/contracts/skills/importer/SKILL.md` | (skip — replaced in Chunks 8–11) |

**Body changes**: For each skill where flags moved out of the hint, ensure the SKILL.md body has an "Invocation" section listing them (see `plugins/spec/skills/execute/SKILL.md` for the canonical shape — `/spec:execute --dry-run`, `/spec:execute --loop`, etc.). Most skills already document flags somewhere; if the documentation is missing, add a 4–8 line `## Invocation` section near the top.

**Acceptance**:

- `rg -n "argument-hint:" plugins/**/SKILL.md` lines never contain `?`, `--`, or `|` (excluding the three contracts skills, which are replaced in Phase 2).
- Every skill that previously had flags in the hint either has no flags or documents them in an "Invocation" section in the body.
- `make checks` passes.

**Dependencies**: None. Can run in parallel with Chunks 1–2.

---

### Chunk 4 — Rewrite `description` per RFC §A.2 table

**RFC**: §A.2

**Scope**: Every SKILL.md with a description that is missing a "Use when…" trigger, contains an RFC citation, or uses a >250-character literal block scalar.

**Per-skill rewrites** (apply verbatim, copying the exact wording from RFC §A.2):

- `plugins/omnia/skills/code-reviewer/SKILL.md` — replace description per the §A.2 row for `omnia/code-reviewer`.
- `plugins/omnia/skills/crate-writer/SKILL.md` — append the trigger tail per §A.2.
- `plugins/omnia/skills/test-writer/SKILL.md` — append the trigger tail per §A.2.
- `plugins/omnia/skills/guest-writer/SKILL.md` — append the trigger tail per §A.2.
- `plugins/plan/skills/sow-writer/SKILL.md` — append the trigger tail per §A.2.
- `plugins/rt/skills/wiretapper/SKILL.md` — append the trigger tail per §A.2.
- `plugins/rt/skills/replay-writer/SKILL.md` — append the trigger tail per §A.2.

**Global cleanups**:

- Drop RFC citations (`RFC-9 §1D`, `RFC-9 §2C`, "Layer 4 umbrella", etc.) from the description fields of `plugins/spec/skills/init/SKILL.md` and `plugins/spec/skills/plan/SKILL.md`. Move the detail (only if not already present) into an "Overview" section in the body.
- Trim the literal-block-scalar (`description: |`) form in `plugins/spec/skills/plan/SKILL.md` to a single-line description ≤250 characters. Relocate any orphaned detail to the body.
- Skip the three `plugins/contracts/skills/*/SKILL.md` files — they are replaced in Phase 2 with new descriptions written from scratch (RFC §A.2 rows for `interfaces/openapi`, `interfaces/asyncapi`, `interfaces/json-schema`).

**Acceptance**:

- Every retained SKILL.md description contains the substring "Use when" (case-insensitive).
- No SKILL.md description references "RFC-" or "Layer N" tokens.
- No SKILL.md description exceeds 1024 characters.
- `make checks` passes.

**Dependencies**: None. Can run in parallel with Chunks 1–3, but defer until after them if you want to minimise merge churn.

---

## Phase 2 — Plugin restructuring

These chunks change directory layout, plugin manifests, and inbound references. They must run sequentially in the order given.

### Chunk 5 — Delete `rt/git-cloner`; inline the clone snippet at the two call sites

**RFC**: §C.1, §F.E (alternative E rejected)

**Scope**:

- Delete the directory `plugins/rt/skills/git-cloner/` (including SKILL.md and any subdirectories).
- Inline a 5-line guarded `git clone` snippet into the two callers that reach for the skill today.

**Edits**:

1. **Delete** `plugins/rt/skills/git-cloner/` recursively.
2. **Inline at `plugins/spec/skills/analyze/SKILL.md`** — find the section that handles `--source <key>=<url>` plan-time discovery. Add a short "Cloning a source tree" subsection containing:

    ```bash
    # Quote DEST and never run rm -rf without verifying the target.
    git clone "$URL" "$DEST"
    test -d "$DEST/.git" && rm -rf "$DEST/.git"   # only if --detach mode is required
    ```

3. **Inline at `plugins/rt/skills/wiretapper/SKILL.md`** — same snippet, framed as legacy-repo bootstrap.
4. **Update inbound prose**:
   - `plugins/rt/README.md` — drop the bullet listing `git-cloner`.
   - `docs/reference/plugins/rt.md` — drop the row/section describing `git-cloner`.
   - `plugins/spec/skills/plan/SKILL.md`, `plugins/spec/skills/define/SKILL.md`, `plugins/spec/skills/extract/references/semantic-search.md`, `plugins/spec/skills/execute/argument-resolution.md`, `plugins/spec/skills/execute/fixtures/e2e-platform-v2/transcript.md`, `schemas/omnia/briefs/plan/discovery.md`, `schemas/vectis/briefs/plan/discovery.md`, `docs/explanation/workspace-tiers.md`, `docs/appendices/glossary.md`, `.cursor/rules/project.mdc`, `plugins/rt/rules/rt.mdc` — replace any reference to `/rt:git-cloner` or `git-cloner` skill with prose that names the inlined snippet location instead. Use `rg -n "git-cloner" -- plugins/ docs/ schemas/ .cursor/` to enumerate.
5. **Marketplace.json**: no change (the manifest lists plugins, not skills).
6. **Archived RFCs and `rfcs/rfc-10-skill-improvements.md`**: do not touch — they are historical references and are explicitly allowlisted.

**Acceptance**:

- `plugins/rt/skills/git-cloner/` no longer exists.
- `rg -n "git-cloner" -- plugins/ docs/ schemas/ .cursor/` returns matches **only** in archived RFCs (under `rfcs/archive/`) and the migration RFC (`rfcs/rfc-10-skill-improvements.md`) and `rfcs/rfc-10-implementation-plan.md` (this file).
- The two callers (`spec/analyze` and `rt/wiretapper`) contain a guarded clone snippet.
- `make checks` passes.

**Dependencies**: None.

---

### Chunk 6 — Rename `plan` plugin to `client`

**RFC**: §C.2, §F.F (alternative F rejected)

**Scope**: Rename the plugin directory, update manifests, set the SKILL.md `name:` to `sow-writer` for now (it gets qualified to `client-sow-writer` in Chunk 14), and sweep inbound references.

**Edits**:

1. **Move directory**: `git mv plugins/plan plugins/client`.
2. **`.cursor-plugin/marketplace.json`**: change the `plan` plugin entry to:

    ```json
    {
      "name": "client",
      "source": "client",
      "description": "Skills to generate client-facing deliverables — Statements of Work, proposals, pricing summaries, and similar artefacts — from Specify artifacts."
    }
    ```

3. **`plugins/client/.cursor-plugin/plugin.json`**: change `"name": "plan"` → `"name": "client"`, `"displayName": "Plan"` → `"displayName": "Client"`, and update `description` to match the marketplace entry. Update `keywords` from `["plan", "sow"]` to `["client", "sow", "deliverables"]`.
4. **`plugins/client/skills/sow-writer/SKILL.md`** frontmatter: leave `name: sow-writer` for now. (It becomes `client-sow-writer` in Chunk 14.) Frontmatter description was already updated in Chunk 4.
5. **`plugins/client/README.md`**: update plugin name and description references from `plan` to `client`.
6. **Inbound docs/refs sweep** — replace references to the old plugin name and slash command. Targets identified by `rg -n "/plan:sow-writer|plugins/plan|\"plan\"" -- plugins/ docs/ schemas/ .cursor/ AGENTS.md README.md`. Specifically:
   - `AGENTS.md` — update mentions of `/plan:sow-writer` to `/client:sow-writer` and any "plan plugin" prose to "client plugin".
   - `README.md` — same updates.
   - `.cursor/rules/project.mdc` — update the `/plan/` plugin section heading and bullet to `/client/`. Update `/plan:sow-writer` → `/client:sow-writer`.
   - `docs/reference/plugins/plan.md` → rename file to `docs/reference/plugins/client.md` (`git mv`); update its content (title, description, slash-command examples).
   - `docs/reference/plugins/index.md` and any plugin index that lists `plan` → list `client`.
   - `docs/explanation/whats-new.md` — note the rename (do **not** add to changelog yet; that lands in Chunk 17).
   - Any fixture or skill body that mentions `/plan:sow-writer`.
7. **Schema briefs that mention `pipeline.plan` are NOT touched**. The phrase "plan" inside Specify schemas refers to `.specify/plan.yaml` and `/spec:plan`, which are unchanged.

**Acceptance**:

- `plugins/plan/` no longer exists; `plugins/client/` does.
- `rg -n "/plan:sow-writer" -- plugins/ docs/ schemas/ .cursor/ AGENTS.md README.md` returns no matches.
- `marketplace.json` lists `client` (not `plan`) as the plugin name.
- `make checks` passes (note: the existing skill check `name === directory name` still holds since `name: sow-writer` matches dir `sow-writer/`).
- `make dev-plugins` still produces a working symlink set.

**Dependencies**: Chunks 1–4 should ideally be done first to avoid frontmatter churn during the move, but this chunk does not strictly require them.

---

### Chunk 7 — Rename `contracts` plugin to `interfaces` (directory + manifests only)

**RFC**: §C.3 (plugin rename portion), §E

**Scope**: Move the plugin directory and update manifests. Skill directories stay at `writer/`, `validator/`, `importer/` for this chunk; they are split into format families in Chunks 8–11.

**Edits**:

1. **Move directory**: `git mv plugins/contracts plugins/interfaces`.
2. **`.cursor-plugin/marketplace.json`**: change the `contracts` plugin entry to:

    ```json
    {
      "name": "interfaces",
      "source": "interfaces",
      "description": "Skills to author, import, and verify interface contracts — OpenAPI, AsyncAPI, JSON Schema, and future API/interface formats — from Specify artifacts."
    }
    ```

3. **`plugins/interfaces/.cursor-plugin/plugin.json`**: change `"name": "contracts"` → `"name": "interfaces"`, `"displayName": "API Contracts"` → `"displayName": "Interface Contracts"`, and update `description` to match the marketplace entry. Keep `keywords` as `["api", "contracts", "openapi", "asyncapi", "json-schema"]` plus `"interfaces"`.
4. **`plugins/interfaces/README.md`**: update plugin name and the slash-command examples from `/contracts:*` to `/interfaces:*`. (The existing skills are still `writer`, `validator`, `importer` in this chunk; the format split is a later chunk. So examples like `/contracts:writer` become `/interfaces:writer` provisionally — they will be re-edited in Chunk 11.)
5. **No skill renames yet.** Frontmatter `name:` for `writer`, `validator`, `importer` stays unchanged.
6. **Skill-internal links unchanged** — relative references from each skill body into `../references/openapi-conventions.md` still resolve because the files moved together.
7. **Active prose under `docs/`, `schemas/`, `AGENTS.md`, `README.md`, `.cursor/rules/project.mdc`** that mentions the **plugin** (`contracts plugin`, `/contracts:*`) — defer mass updates to Chunk 13. This chunk stops at the directory + manifest move so that the next four chunks can proceed independently.

**Critical: do NOT change**:

- `schemas/contracts/` (schema directory)
- `.specify/contracts/` references in any prose
- `contracts@v1` schema id
- The brief id `contracts` (frontmatter `id: contracts` in brief markdown)
- `contracts.*` validation rule ids
- Registry roles `contracts.{produces,consumes,imports}`

**Acceptance**:

- `plugins/contracts/` no longer exists; `plugins/interfaces/` does.
- `marketplace.json` lists `interfaces` (not `contracts`) as the plugin name.
- `plugins/interfaces/skills/{writer,validator,importer}/` all still exist and still have `name: writer|validator|importer` in their frontmatter.
- `rg -n "schemas/contracts" -- ` still returns the same matches as before this chunk (schema dir is unchanged).
- `make checks` passes.

**Dependencies**: Chunk 1 (license drop) recommended first to avoid double-touching files.

---

### Chunk 8 — Author `plugins/interfaces/skills/openapi/` (new format-family skill)

**RFC**: §C.3 (per-format skill body shape), §A.2 (description)

**Scope**: Create the new `openapi` skill directory with SKILL.md and three siblings (`author.md`, `importer.md`, `verifier.md`). Source material from the existing `plugins/interfaces/skills/{writer,validator,importer}/SKILL.md` files — keep only OpenAPI-specific guidance in this skill.

**Edits**:

1. **Create directory** `plugins/interfaces/skills/openapi/` with files:
   - `SKILL.md`
   - `author.md`
   - `importer.md`
   - `verifier.md`
2. **`SKILL.md` frontmatter**:

    ```yaml
    ---
    name: openapi
    description: Authors, imports, and verifies OpenAPI 3.1 HTTP API contracts for Specify changes, including path operations, request and response schemas, parameters, auth, examples, and baseline deltas. Use when the contracts brief needs an HTTP API contract, when an operator supplies or asks for an OpenAPI document, or when verifying OpenAPI compatibility after a merge.
    argument-hint: "[change-dir]"
    ---
    ```

   (`name: openapi` matches the directory name and passes the existing dirname check. Chunk 14 qualifies it to `interfaces-openapi`.)

3. **`SKILL.md` body** — open with a Critical Path quick-reference (5–7 bullets), then an intent-dispatch table:

    | Intent | Trigger | Sibling |
    |---|---|---|
    | Author or extend the OpenAPI document from a spec | contracts brief during `/spec:define`; operator extending the baseline for new HTTP interactions | `author.md` |
    | Import or normalise an external OpenAPI document | operator drops an OpenAPI file into a change's `contracts/http/` directory | `importer.md` |
    | Verify internal consistency or run the cross-project consumer check | contracts brief post-merge (RFC-9 §3B); operator invoking validation against an existing OpenAPI artefact | `verifier.md` |

   Keep SKILL.md ≤300 lines. Push deep guidance into the siblings.

4. **`author.md`** — copy-and-prune OpenAPI-relevant content from `plugins/interfaces/skills/writer/SKILL.md` and `plugins/interfaces/references/openapi-conventions.md`. Cover: spec → operations mapping, schema reuse from `contracts/schemas/`, baseline-delta computation rules, examples, auth.
5. **`importer.md`** — copy-and-prune content from `plugins/interfaces/skills/importer/SKILL.md` and `plugins/interfaces/skills/importer/references/{format-detection,upgrade-rules}.md`. Cover: detect 3.0 vs 3.1, upgrade to 3.1, decompose inline schemas into `contracts/schemas/`, inject Specify metadata.
6. **`verifier.md`** — copy-and-prune content from `plugins/interfaces/skills/validator/SKILL.md`. Cover: `$ref` resolution, schema metadata completeness, binding coverage, single-mode and cross-project verifier modes (the `--mode {single, cross-project}` distinction becomes an internal flag here, not a top-level skill, per RFC §C.3).

**Hard rules**:

- Keep this skill OpenAPI-only. No AsyncAPI or JSON Schema-only guidance.
- Reuse existing references where possible: link to `../../references/openapi-conventions.md` (still under `plugins/interfaces/references/`). Do not move or duplicate content yet — Chunk 11 handles the format-neutral references factor-out.

**Acceptance**:

- `plugins/interfaces/skills/openapi/{SKILL.md,author.md,importer.md,verifier.md}` all exist.
- Frontmatter validates against `schemas/skill.schema.json`.
- `SKILL.md` is ≤300 lines, has a Critical Path block, and an intent-dispatch table.
- `rg -nL "openapi" plugins/interfaces/skills/openapi/` shows OpenAPI is referenced throughout.
- `make checks` passes.

**Dependencies**: Chunk 7 must land first (so the directory is `interfaces/`).

---

### Chunk 9 — Author `plugins/interfaces/skills/asyncapi/` (new format-family skill)

**RFC**: §C.3, §A.2 (description)

**Scope**: Same shape as Chunk 8, for AsyncAPI 3.0.

**Edits**:

1. **Create directory** `plugins/interfaces/skills/asyncapi/` with files:
   - `SKILL.md`
   - `author.md`
   - `importer.md`
   - `verifier.md`
2. **`SKILL.md` frontmatter**:

    ```yaml
    ---
    name: asyncapi
    description: Authors, imports, and verifies AsyncAPI 3.0 event, pub/sub, stream, and WebSocket-style contracts for Specify changes, including channels, messages, bindings, producers, consumers, and schema references. Use when the contracts brief needs an evented interface contract, when an operator supplies or asks for an AsyncAPI document, or when verifying AsyncAPI compatibility after a merge.
    argument-hint: "[change-dir]"
    ---
    ```

3. **`SKILL.md` body** — Critical Path (5–7 bullets) + intent-dispatch table specialised to AsyncAPI:

    | Intent | Trigger | Sibling |
    |---|---|---|
    | Author or extend the AsyncAPI document from a spec | contracts brief during `/spec:define`; operator extending the baseline for new evented interactions | `author.md` |
    | Import or normalise an external AsyncAPI document | operator drops an AsyncAPI file into a change's `contracts/messages/` directory | `importer.md` |
    | Verify internal consistency or run the cross-project consumer check | contracts brief post-merge (RFC-9 §3B); operator invoking validation against an existing AsyncAPI artefact | `verifier.md` |

4. **`author.md`** — channels, messages, bindings, producers, consumers, schema reuse, baseline-delta computation. Source material from existing `writer/SKILL.md` and `references/asyncapi-conventions.md`.
5. **`importer.md`** — AsyncAPI version detection and upgrade rules (sources: `importer/SKILL.md` + `importer/references/`).
6. **`verifier.md`** — bindings, message validation, schema metadata, single + cross-project modes.

**Acceptance**: same shape as Chunk 8, scoped to AsyncAPI.

**Dependencies**: Chunk 7. Independent of Chunks 8 and 10.

---

### Chunk 10 — Author `plugins/interfaces/skills/json-schema/` (new format-family skill)

**RFC**: §C.3, §A.2 (description)

**Scope**: Same shape as Chunks 8–9, for standalone JSON Schema documents.

**Edits**:

1. **Create directory** `plugins/interfaces/skills/json-schema/` with files:
   - `SKILL.md`
   - `author.md`
   - `importer.md`
   - `verifier.md`
2. **`SKILL.md` frontmatter**:

    ```yaml
    ---
    name: json-schema
    description: Authors, imports, and verifies standalone JSON Schema documents shared by OpenAPI, AsyncAPI, and other interface contracts. Use when a Specify change needs reusable payload schemas, when an operator supplies schema files without a protocol wrapper, or when validating schema compatibility across generated interface contracts.
    argument-hint: "[change-dir]"
    ---
    ```

3. **`SKILL.md` body** — Critical Path (5–7 bullets) + intent-dispatch table specialised to JSON Schema:

    | Intent | Trigger | Sibling |
    |---|---|---|
    | Author or extend reusable schemas from a spec | contracts brief during `/spec:define`; operator extending the baseline for new payload types | `author.md` |
    | Import or normalise external schema files | operator drops schema files into a change's `contracts/schemas/` directory | `importer.md` |
    | Verify `$ref` consistency, metadata, and cross-format consumer compatibility | contracts brief post-merge; mixed-format change verification | `verifier.md` |

4. **`author.md`** — `$id` assignment, one-type-per-file decomposition, schema-file naming, vocabulary for shared payloads (per RFC §C.3 mixed-format ordering, this skill owns these decisions).
5. **`importer.md`** — schema file detection, normalisation, upgrade rules.
6. **`verifier.md`** — `$ref` resolution, metadata completeness, duplicate-`$id` detection across the change, cross-format consumer compatibility.

**Mixed-format ordering note** (per RFC §C.3): the json-schema skill must run **before** OpenAPI and AsyncAPI in mixed-format briefs because it owns shared schema vocabulary. Document this constraint in `SKILL.md`.

**Acceptance**: same shape as Chunks 8–9.

**Dependencies**: Chunk 7. Independent of Chunks 8 and 9.

---

### Chunk 11 — Factor format-neutral references; delete the three lifecycle skills

**RFC**: §C.3 (shared references; deletion of writer/validator/importer)

**Scope**:

- Move format-neutral content into `plugins/interfaces/references/`.
- Delete `plugins/interfaces/skills/{writer,validator,importer}/` directories.
- Reconcile cross-links across the three new format skills.

**Edits**:

1. **Audit** `plugins/interfaces/skills/{writer,validator,importer}/SKILL.md` (and their `references/`). Bucket each section into:
   - **Format-neutral** (artifact layout, baseline-vs-delta rules, generic `$ref` conventions, format detection logic, import upgrade policy summary, report shape, cross-project compatibility vocabulary) → move to `plugins/interfaces/references/`.
   - **Format-specific** (already covered by the per-format `author.md` / `importer.md` / `verifier.md` files in Chunks 8–10) → discard.
2. **Author or extend** these shared references under `plugins/interfaces/references/`:
   - `artifact-structure.md` (already exists; extend)
   - `baseline-vs-delta.md` (new; cross-format rules)
   - `import-upgrade-policy.md` (new; framework for per-format upgrade rules)
   - `report-shape.md` (new; verifier output JSON shape, single + cross-project modes)
   - `cross-project-compatibility.md` (new; vocabulary used by all three format verifiers)
   - Existing `openapi-conventions.md`, `asyncapi-conventions.md`, `json-schema-conventions.md` — leave as format-specific references; link from the corresponding `author.md` / `verifier.md` siblings.
3. **Update cross-links** in the three format-skill `SKILL.md` files and their siblings to point at the new shared references using relative paths (`../../references/<file>.md`).
4. **Delete** `plugins/interfaces/skills/writer/`, `plugins/interfaces/skills/validator/`, `plugins/interfaces/skills/importer/` recursively.
5. **Update** `plugins/interfaces/README.md` to list the three new skills and their purposes (replacing any reference to writer/validator/importer).

**Acceptance**:

- `plugins/interfaces/skills/{writer,validator,importer}/` no longer exist.
- `plugins/interfaces/skills/{openapi,asyncapi,json-schema}/` exist with all four files each.
- `plugins/interfaces/references/` contains shared references and resolves all relative links from the format skills.
- `make checks` passes — including `checkSkillReferences` (verifies relative `references/` and `examples/` links from each SKILL.md resolve).

**Dependencies**: Chunks 8, 9, 10 must all be complete.

---

### Chunk 12 — Brief retargets across schemas

**RFC**: §C.3 (brief-body retargets)

**Scope**: Update active **prose** inside Specify brief markdown to reference the new `/interfaces:*` slash commands instead of `/contracts:*`. Brief frontmatter (`id`, `description`, `generates`, `needs`) does **not** mention skills today and stays untouched.

**Targets** (identified by `rg -n "/contracts:writer|/contracts:validator|/contracts:importer" -- schemas/`):

- `schemas/contracts/briefs/contracts.md`
- `schemas/contracts/briefs/build.md`
- `schemas/contracts/briefs/tasks.md`
- `schemas/omnia/briefs/contracts.md`
- `schemas/vectis/briefs/contracts.md`
- Any other brief surfaced by the grep above.

**Retargeting rules** (per RFC §C.3):

- `/contracts:writer` → choose based on the brief context:
  - HTTP/resource APIs: `/interfaces:openapi` (with prose intent "author the minimal OpenAPI delta").
  - Evented/pub-sub/streaming: `/interfaces:asyncapi`.
  - Reusable payload schemas without a protocol wrapper: `/interfaces:json-schema`.
  - Mixed-format briefs: keep both/all three slash commands and add the explicit ordering rule from RFC §C.3 ("run `/interfaces:json-schema` first when shared payload vocabulary is present, then `/interfaces:openapi` for HTTP, then `/interfaces:asyncapi` for events").
- `/contracts:validator` → `/interfaces:<format>` `verifier.md` intent. For cross-project consumer-check briefs, document `--mode cross-project` as a verifier flag (per RFC §C.3, the mode is now an internal verifier option).
- `/contracts:importer` → `/interfaces:<format>` with prose intent "import or normalise the supplied document".

**HTML skill directives** (`<!-- skill: contracts:writer -->`, etc.) — these appear in `schemas/`. Replace them with the appropriate `<!-- skill: interfaces:openapi -->` / `<!-- skill: interfaces:asyncapi -->` / `<!-- skill: interfaces:json-schema -->`. Skill directives are validated by `scripts/checks.ts` (`checkSkillDirectives`), so they must reference real plugin/skill pairs after this chunk completes.

**Critical: do NOT change**:

- The brief id (frontmatter `id: contracts` stays).
- The schema id `contracts@v1` — stays.
- File paths under `schemas/contracts/` — stay.
- The brief description text wording for `id`/`description`/`needs` — those don't mention skills.

**Acceptance**:

- `rg -n "/contracts:writer|/contracts:validator|/contracts:importer" -- schemas/` returns no matches.
- `rg -n "<!-- skill: contracts:" -- schemas/ plugins/ docs/` returns no matches.
- `make checks` passes (especially `checkSkillDirectives` and `checkSchemaIntegrity`).

**Dependencies**: Chunks 8–11 (the new format skills must exist before directives can reference them).

---

### Chunk 13 — Inbound docs/refs sweep for the three renames

**RFC**: §C.1, §C.2, §C.3

**Scope**: Update all remaining active prose under `docs/`, `AGENTS.md`, `README.md`, `.cursor/rules/project.mdc`, fixtures, and skill bodies that mention the renamed/deleted slash commands.

**Find candidates** (run from repo root):

```bash
rg -n "/plan:sow-writer|/contracts:writer|/contracts:validator|/contracts:importer|/rt:git-cloner" \
  -- plugins/ docs/ schemas/ .cursor/ AGENTS.md README.md
```

Active files that are expected to need edits (based on pre-RFC grep):

- `AGENTS.md`
- `README.md`
- `.cursor/rules/project.mdc`
- `docs/explanation/whats-new.md`
- `docs/explanation/workspace-tiers.md`
- `docs/appendices/glossary.md`
- `docs/appendices/troubleshooting.md`
- `docs/how-to/cross-repo-contracts.md`
- `docs/how-to/resolve-cross-project-contract-warnings.md`
- `docs/tutorials/cross-repo-initiative.md`
- `docs/reference/quick-reference.md`
- `docs/reference/plugins/contracts.md` → rename to `docs/reference/plugins/interfaces.md` (`git mv`); update content
- `docs/reference/plugins/index.md` (if it lists `contracts`)
- `docs/reference/plugins/rt.md` (already touched in Chunk 5; double-check)
- `docs/reference/schemas/contracts.md` — rename: keep file at `contracts.md` because the schema is still `contracts@v1`. Update prose-internal slash-command examples only.
- `docs/reference/schemas/index.md`
- `docs/reference/change-skills/build.md`
- Any plugin README that references the old commands.
- `plugins/spec/skills/execute/output-format.md`, `fixtures.md`, `multi-repo.md`, `SKILL.md` — already grep-listed, update slash commands.
- Fixture transcripts and READMEs under `plugins/spec/skills/execute/fixtures/cross-project-contract-warning/` — re-baseline transcripts that exercise the old surface.

**Allowlist** (do **not** touch):

- `rfcs/archive/*` — historical RFCs.
- `rfcs/rfc-10-skill-improvements.md` — the source RFC.
- `rfcs/rfc-10-implementation-plan.md` — this file.

**Edit rules**:

- Replace `/plan:sow-writer` → `/client:sow-writer`.
- Replace `/contracts:writer|validator|importer` → the appropriate `/interfaces:openapi|asyncapi|json-schema` based on context (HTTP / evented / shared-schema). Where the context is a generic example of "running a contracts skill", show all three options or the most representative one.
- Replace `/rt:git-cloner` → "an inlined `git clone` snippet" or remove the bullet entirely if the surrounding sentence becomes redundant.
- Where the prose mentions "contracts plugin" as a Cursor surface, use "interfaces plugin"; where it mentions "the `contracts` schema/brief/artifacts/baseline directory", keep `contracts` (those are persisted identifiers).
- Where the prose mentions "plan plugin" referring to the SoW generator, use "client plugin".

**Acceptance**:

- `rg -n "/plan:sow-writer|/contracts:writer|/contracts:validator|/contracts:importer|/rt:git-cloner" -- plugins/ docs/ schemas/ .cursor/ AGENTS.md README.md` returns no matches.
- `docs/reference/plugins/plan.md` no longer exists; `docs/reference/plugins/client.md` does (renamed in Chunk 6 — verify).
- `docs/reference/plugins/contracts.md` no longer exists; `docs/reference/plugins/interfaces.md` does.
- `make checks` passes (markdown link integrity, skill directive validation).

**Dependencies**: Chunks 5, 6, 7, 8, 9, 10, 11, 12.

---

## Phase 3 — Body factoring and splits

These chunks shrink oversized SKILL.md bodies and remove duplicated normative prose.

### Chunk 14 — Author the phase-outcome reference; replace duplicated sections in 4 phase skills

**RFC**: §B.3

**Scope**:

- Create `plugins/spec/references/phase-outcome-contract.md` with the parameterised contract.
- In each of `plugins/spec/skills/{define,build,merge,drop}/SKILL.md`, replace the three duplicated sections with a 4-line shim plus a phase-specific delta block.

**Edits**:

1. **Create** `plugins/spec/references/phase-outcome-contract.md` containing:
   - Outcome values: `success`, `failure`, `deferred` and what each means.
   - Journal kinds: `question`, `failure`, `recovery`.
   - The plan-mutation allow/forbid table (which phases may add / amend / transition plan entries mid-run).
   - The verbatim-`summary` rule (the driver copies `outcome.summary` byte-for-byte into `--reason`).
   - Success/failure/deferred semantics shared across phases.
   - Source the union of the existing `## Phase outcome contract`, `## Journal entries during the run`, and `## Mutating the plan mid-run` sections from `define`, `build`, and `merge`. Resolve any wording differences in favour of the most precise phrasing.
2. **Replace** the three duplicated sections in each phase SKILL.md with the shim shape from RFC §B.3:

    ```markdown
    ## Phase outcome contract

    This skill is the **<phase>** phase of the `/spec:execute` driver loop.
    The shared phase contract — outcome values, journal kinds, plan-mutation rules,
    the verbatim-`summary` rule, and the success/failure/deferred semantics — is
    authored once at [`../../references/phase-outcome-contract.md`](../../references/phase-outcome-contract.md).

    This phase's outcome-specific deltas:

    - `success` — <phase-specific success criteria>
    - `failure` — <phase-specific failure modes>
    - `deferred` — <phase-specific deferral triggers>
    ```

3. **Phase-specific deltas to preserve**:
   - `define`: success = artifacts complete, no `[unknown]` blockers; failure = brief halted before all artifacts written; deferred = upstream input missing.
   - `build`: success = all tasks marked complete, validation green; failure = test/build halt; deferred = blocked on a question.
   - `merge`: success = baseline merge complete, archive moved (success path is uniquely **CLI-stamped** — do not call `outcome set` per `merge/SKILL.md` line ~199); failure = conflict detection halt; deferred = pending a precondition.
   - `drop`: success = change archived with `dropped` status; failure = lifecycle violation; deferred = (rare) deferred drop.
4. **`drop/SKILL.md`** is shorter and uses `--reason` rather than the explicit `outcome set` form. Keep the existing `--reason` discussion in that skill, but factor the shared semantics out to the reference. Replace the prose that overlaps with the reference and keep the `--reason` specifics.
5. **Cross-link**: leave existing internal references like "see §Phase outcome contract above" intact in `merge/SKILL.md` (line ~199) — the section header still exists, it just now contains the shim.

**Acceptance**:

- `plugins/spec/references/phase-outcome-contract.md` exists with the unified contract (~100 lines).
- Each of `define`, `build`, `merge`, `drop` SKILL.md contains the 4-line shim plus a 3–6-bullet delta block.
- Total LOC removed across the four phase skills is **at least ~130 net** (the RFC's optimistic ~240 estimate is mathematically capped near ~131 because `drop` had no `## Phase outcome contract` section before and now has to gain one). The chunk's success criterion is the factor-out itself, not a literal LOC count.
- `make checks` passes (markdown link integrity, skill reference resolution).

**Dependencies**: Chunks 1–4 should land first to avoid frontmatter churn.

---

### Chunk 15 — Split `omnia/code-reviewer` (691 → ≤300)

**RFC**: §B.1

**Scope**: Factor `plugins/omnia/skills/code-reviewer/SKILL.md` into SKILL.md plus four siblings, per RFC §B.1.

**Target layout**:

```text
plugins/omnia/skills/code-reviewer/
├── SKILL.md            # critical path + invocation + output shape (≤300 lines)
├── categories.md       # SEC-/COR-/QUA-/UNI- check libraries
├── team-protocol.md    # specialist spawn / antagonist / synthesis rules
├── auto-fix.md         # --fix scope, success-rate, regression guard
├── output.md           # REVIEW.md template + finding-ID conventions
└── references/         # (existing; unchanged)
```

**Edits**:

1. **Audit current `SKILL.md`** (691 lines). Identify the four sections to extract:
   - SEC-/COR-/QUA-/UNI- check categories → `categories.md`
   - Specialist spawn / antagonist / synthesis protocol → `team-protocol.md`
   - `--fix` scope, success-rate gating, regression guards → `auto-fix.md`
   - REVIEW.md template + finding-ID conventions → `output.md`
2. **Create the four sibling files**, each opening with a 1–2 sentence "When to read this" header that names the trigger (per RFC's Critical Path discipline).
3. **Rewrite `SKILL.md`** to contain:
   - Frontmatter (already updated in Chunks 1–4).
   - A Critical Path quick-reference (5–7 bullets) at the top.
   - An "Invocation" section (flags moved out of the `argument-hint`).
   - A short overview of the review pipeline.
   - References to the four sibling files at the points where Claude needs to load them.
   - The output shape summary (full template lives in `output.md`).
4. **Update internal links**: any reference inside `references/` that pointed to the old monolithic SKILL.md sections must be retargeted to the new sibling files.
5. **Verify the Critical Path block** names every algorithmic step so an operator scanning SKILL.md alone can confirm the pipeline.

**Acceptance**:

- `plugins/omnia/skills/code-reviewer/SKILL.md` body (post-frontmatter) is ≤300 lines.
- The four siblings exist and contain the extracted content.
- `make checks` passes (markdown links, skill references).

**Dependencies**: Chunks 1–4 (frontmatter), Chunk 14 (project.mdc convention is documented but not enforced yet). Independent of Chunk 16.

---

### Chunk 16 — Split `omnia/crate-writer` (507 → ≤450)

**RFC**: §B.1

**Scope**: Push the "Hard Rules" enumeration and "Authority Hierarchy" body out of `plugins/omnia/skills/crate-writer/SKILL.md` into a new `rules.md`. The skill already has `references/` and `examples/` carrying most depth.

**Target layout**:

```text
plugins/omnia/skills/crate-writer/
├── SKILL.md       # critical path + mode-dispatch table + artifact-mapping (≤450)
├── rules.md       # Hard Rules + Authority Hierarchy
├── references/    # existing
└── examples/      # existing
```

**Edits**:

1. **Identify** the "Hard Rules" and "Authority Hierarchy" sections in current SKILL.md.
2. **Create** `rules.md`, copy those sections verbatim, and add a "When to read this" header.
3. **Trim SKILL.md** to:
   - Frontmatter.
   - Critical Path quick-reference.
   - Mode-dispatch table (greenfield vs incremental).
   - Artifact-mapping section.
   - A pointer to `rules.md` for the binding constraints.
   - Pointers to `references/` and `examples/` as today.
4. **Update internal links** so any prior cross-reference to "see Hard Rules above" becomes a relative link to `rules.md`.

**Acceptance**:

- SKILL.md body ≤450 lines.
- `rules.md` contains the extracted sections.
- `make checks` passes.

**Dependencies**: Chunks 1–4. Independent of Chunk 15.

---

### Chunk 17 — Critical Path + recount for near-limit skills

**RFC**: §B.2

**Scope**: For every SKILL.md within 25 lines of the 500-line ceiling, add a Critical Path quick-reference at the top **only together with an offsetting extraction or deletion**. Recount; if the result is ≥500 lines, extract a sibling reference in the same pass.

**In-scope files** (line counts from pre-RFC inventory):

- `plugins/vectis/skills/core-writer/SKILL.md` (495)
- `plugins/vectis/skills/android-writer/SKILL.md` (490)
- `plugins/vectis/skills/android-reviewer/SKILL.md` (489)
- `plugins/vectis/skills/core-reviewer/SKILL.md` (484)
- `plugins/client/skills/sow-writer/SKILL.md` (477; renamed from `plan/` in Chunk 6)

**Edits per file**:

1. Add a "Critical Path (Quick Reference)" 5–7 bullet block at the top, summarising the algorithm.
2. Identify a section that can be extracted to a sibling reference of equal or greater length than the new Critical Path block (so the file does not grow). Good candidates:
   - For writers: a "Hard Rules" / "Authority Hierarchy" section → `rules.md`.
   - For reviewers: a check-categories table or specialist-team protocol → `categories.md` / `team-protocol.md`.
   - For sow-writer: a template structure block → `template.md`.
3. Recount lines (`wc -l SKILL.md`). If body (post-frontmatter) is ≥500, extract a second sibling. Do **not** ship a SKILL.md ≥500 lines.

**Acceptance**:

- All five files have a Critical Path block at the top of SKILL.md.
- All five SKILL.md bodies (post-frontmatter) are ≤499 lines.
- Each file has at least one new sibling reference linked from SKILL.md.
- `make checks` passes.

**Dependencies**: Chunks 1–4 (frontmatter); Chunk 6 (the sow-writer file is now under `plugins/client/`); Chunk 14 (project.mdc has the Critical Path convention — but this is not enforced until Chunk 19).

---

## Phase 4 — Schema + checks + name qualification + house style

### Chunk 18 — Update `schemas/skill.schema.json` to the new frontmatter shape

**RFC**: §D (frontmatter shape)

**Scope**: Tighten the JSON Schema for SKILL.md frontmatter to match the post-RFC conventions.

**Edits**:

- Required: `name`, `description`.
- Optional: `argument-hint`, `allowed-tools`.
- Remove from `properties`: `license`, `compatibility`, `metadata`, `disable-model-invocation`, `when_to_use`, `user-invocable`, `paths`.
- Keep `additionalProperties: false`.
- Tighten `name`:
  - `pattern`: keep `^[a-z][a-z0-9-]*$` (Anthropic syntax).
  - `maxLength`: 64.
  - Update `description` to note it must be plugin-qualified (the per-plugin prefix invariant is enforced in `scripts/checks.ts`, not here, because JSON Schema cannot see the surrounding directory).
- `description`:
  - `maxLength`: 1024.
  - `minLength`: 10.
- `argument-hint`:
  - Update the JSON Schema `description` to point at Cursor placeholder convention: short, single-line, `<>` for required, `[]` for optional, no flags.
  - Optional: add a `pattern` that forbids `?`, `--`, `|` (note: `|` is also used inside square brackets like `<a|b|c>`; if the pattern is too aggressive, leave the constraint to `scripts/checks.ts`).

**Acceptance**:

- `schemas/skill.schema.json` validates every existing SKILL.md frontmatter (after Chunks 1–17 have removed `license` and any other obsolete fields).
- `make checks` passes (the existing `validateSkillFrontmatter` runs the schema against every SKILL.md).

**Dependencies**: All of Chunks 1–17 (so no SKILL.md still carries `license:` or other forbidden fields).

---

### Chunk 19 — Apply RFC §A.1 name qualification to every SKILL.md + update `scripts/checks.ts`

**RFC**: §A.1, §D (mechanical checks)

**Scope**: This is a **single combined chunk** because the name change and the check rule update are coupled: changing one without the other breaks `make checks`.

**Edits — frontmatter rewrites** (apply this verbatim mapping; skip the three contracts/* and the deleted git-cloner):

| Path | Old `name:` | New `name:` |
|---|---|---|
| `plugins/spec/skills/init/SKILL.md` | `init` | `specify-init` |
| `plugins/spec/skills/define/SKILL.md` | `define` | `specify-define` |
| `plugins/spec/skills/build/SKILL.md` | `build` | `specify-build` |
| `plugins/spec/skills/merge/SKILL.md` | `merge` | `specify-merge` |
| `plugins/spec/skills/drop/SKILL.md` | `drop` | `specify-drop` |
| `plugins/spec/skills/extract/SKILL.md` | `extract` | `specify-extract` |
| `plugins/spec/skills/analyze/SKILL.md` | `analyze` | `specify-analyze` |
| `plugins/spec/skills/plan/SKILL.md` | `plan` | `specify-plan` |
| `plugins/spec/skills/execute/SKILL.md` | `execute` | `specify-execute` |
| `plugins/omnia/skills/crate-writer/SKILL.md` | `crate-writer` | `omnia-crate-writer` |
| `plugins/omnia/skills/test-writer/SKILL.md` | `test-writer` | `omnia-test-writer` |
| `plugins/omnia/skills/guest-writer/SKILL.md` | `guest-writer` | `omnia-guest-writer` |
| `plugins/omnia/skills/code-reviewer/SKILL.md` | `code-reviewer` | `omnia-code-reviewer` |
| `plugins/vectis/skills/core-writer/SKILL.md` | `core-writer` | `vectis-core-writer` |
| `plugins/vectis/skills/core-reviewer/SKILL.md` | `core-reviewer` | `vectis-core-reviewer` |
| `plugins/vectis/skills/ios-writer/SKILL.md` | `ios-writer` | `vectis-ios-writer` |
| `plugins/vectis/skills/ios-reviewer/SKILL.md` | `ios-reviewer` | `vectis-ios-reviewer` |
| `plugins/vectis/skills/android-writer/SKILL.md` | `android-writer` | `vectis-android-writer` |
| `plugins/vectis/skills/android-reviewer/SKILL.md` | `android-reviewer` | `vectis-android-reviewer` |
| `plugins/vectis/skills/design-system-writer/SKILL.md` | `design-system-writer` | `vectis-design-system-writer` |
| `plugins/vectis/skills/test-writer/SKILL.md` | `test-writer` | `vectis-test-writer` |
| `plugins/vectis/skills/template-updater/SKILL.md` | `template-updater` | `vectis-template-updater` |
| `plugins/rt/skills/wiretapper/SKILL.md` | `wiretapper` | `rt-wiretapper` |
| `plugins/rt/skills/replay-writer/SKILL.md` | `replay-writer` | `rt-replay-writer` |
| `plugins/client/skills/sow-writer/SKILL.md` | `sow-writer` | `client-sow-writer` |
| `plugins/interfaces/skills/openapi/SKILL.md` | `openapi` | `interfaces-openapi` |
| `plugins/interfaces/skills/asyncapi/SKILL.md` | `asyncapi` | `interfaces-asyncapi` |
| `plugins/interfaces/skills/json-schema/SKILL.md` | `json-schema` | `interfaces-json-schema` |

**Edits — `scripts/checks.ts` (function `validateSkillFrontmatter` and adjacent)**:

1. **Replace the existing `name === dirName` check** with these invariants:
   - `name` must satisfy `^[a-z][a-z0-9-]*$` (already in the JSON Schema).
   - `name` must start with the containing plugin's directory name plus `-`, **subject to a single override**: the `spec/` plugin uses the `specify-` prefix per RFC §A.1 (the plugin directory and slash-command namespace stay `spec` because operators call `/spec:init` etc., but the global skill `name:` carries the operator-facing product name `specify-`). Implement this with a small `PREFIX_OVERRIDES = { spec: "specify" }` map in `scripts/checks.ts`. Compute the plugin dir as the path component immediately under `plugins/` and look it up in the override map before falling back to the dir name.
   - `name` must be globally unique across the entire `plugins/**/SKILL.md` tree.
2. **Add line-count check**: SKILL.md body (post-frontmatter, i.e. everything after the closing `---`) must be ≤500 lines. Fail with a message naming the file and the line count.
3. **Add description-length check**: `description` must be ≤1024 chars (already in JSON Schema; assert in TS as a defence-in-depth check that surfaces the count clearly).
4. **Add argument-hint shape check**: `argument-hint` value must not contain `?`, `--`, or `|`.
5. **Add no-license check**: SKILL.md frontmatter must not contain a `license:` key.
6. **Add retired-slash-command check**: Scan all `.md` files under `plugins/`, `docs/`, `schemas/`, `.cursor/`, `AGENTS.md`, `README.md` for any of: `/plan:sow-writer`, `/rt:git-cloner`, `/contracts:writer`, `/contracts:validator`, `/contracts:importer`, `/contracts:management`. Maintain an explicit allowlist:

    ```typescript
    const RETIRED_SLASH_ALLOWLIST = new Set<string>([
      "rfcs/rfc-10-skill-improvements.md",
      "rfcs/rfc-10-implementation-plan.md",
      // archived RFCs match by glob: rfcs/archive/**
    ]);
    ```

   Skip files under `rfcs/archive/` entirely (treat them as historical).
7. **Wire the new checks into the main `await Promise.all([...])` block** at the bottom of `scripts/checks.ts`.

**Inbound references**: `plugins/<plugin>/.cursor-plugin/plugin.json` `name` field is independent (it is the plugin name, not the skill name). Do not touch.

**Acceptance**:

- Every SKILL.md frontmatter has a globally unique `name:` that starts with its plugin directory name + `-`.
- `scripts/checks.ts` enforces all invariants in §D.
- `make checks` passes.
- Manually verified: temporarily revert one rename and re-run `make checks` to confirm the new check fails as expected; then re-apply the rename.

**Dependencies**: All of Chunks 1–18.

---

### Chunk 20 — House-style codification: project.mdc + skill-authoring.md + AGENTS.md link

**RFC**: §B.2 (Critical Path as house style), §D (codification location)

**Scope**: Document the conventions in two places.

**Edits**:

1. **`.cursor/rules/project.mdc`** — add a new top-level section "## Skill authoring conventions" containing:
   - Frontmatter shape: required `name`, `description`; optional `argument-hint`, `allowed-tools`. Forbidden: `license`, `compatibility`, `metadata`, `disable-model-invocation`, `when_to_use`, `user-invocable`, `paths`.
   - Naming: globally unique, plugin-qualified, lowercase + hyphens, gerund or action-oriented or noun-phrase. Prefer gerunds for new skills; preserve product verbs and artifact nouns when more discoverable.
   - Description: third person, includes both *what* and *when*, ≤1024 chars.
   - Argument-hint: Cursor placeholder text, single short hint, `<>` / `[]` brackets, no flag names, no `?` suffix.
   - Critical Path: every SKILL.md ≥150 lines must lead with a "Critical Path (Quick Reference)" block of 5–7 bullets.
   - Body length: SKILL.md must stay under 500 lines; longer content goes in sibling files linked one level deep.
   - Link to the longer-form companion: `docs/explanation/skill-authoring.md`.
   - Link to Anthropic docs: [overview](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview), [best practices](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices).
2. **Create `docs/explanation/skill-authoring.md`** with the long-form rationale: discovery model (two-stage loading), why metadata is precious, examples of good and bad descriptions, the progressive-disclosure pattern, the phase-outcome reference as a worked example. Length target: 200–400 lines.
3. **`AGENTS.md`** — add a one-line bullet linking to `docs/explanation/skill-authoring.md` in the "Workflow overview" or a near section.

**Acceptance**:

- `.cursor/rules/project.mdc` has a "Skill authoring conventions" section.
- `docs/explanation/skill-authoring.md` exists and is linked from `AGENTS.md` and from `project.mdc`.
- `make checks` passes (markdown link integrity).

**Dependencies**: Chunks 18, 19 (so the conventions match the just-landed code).

---

## Phase 5 — Cross-repo + release

### Chunk 21 — `specify-cli` cross-repo sweep

**RFC**: §E

**Repository**: `/Users/andrewweston/rust/github.com/augentic/specify-cli` (sibling to this repo).

**Scope**: Update tests, fixtures, and examples that mention retired skill directives. Do **not** touch `contracts@v1`, `.specify/contracts/`, contract validation rule ids, merge behaviour, or workspace contract distribution.

**Edits**:

1. **Find candidates**:

    ```bash
    cd /Users/andrewweston/rust/github.com/augentic/specify-cli
    rg -n "/plan:sow-writer|/contracts:writer|/contracts:validator|/contracts:importer|/rt:git-cloner|<!-- skill: contracts:|<!-- skill: plan:|<!-- skill: rt:git-cloner -->"
    ```

2. **Apply the same retargeting rules** as Chunks 5, 6, 12, 13:
   - `<!-- skill: contracts:writer -->` → `<!-- skill: interfaces:openapi -->` (or `:asyncapi` / `:json-schema` based on context).
   - `<!-- skill: plan:sow-writer -->` → `<!-- skill: client:sow-writer -->`.
   - `<!-- skill: rt:git-cloner -->` → remove or replace with a prose note.
3. **Pre-existing grep hit**: `schemas/omnia/briefs/specs.md` contains `git-cloner`. Replace that mention with a prose note about inlining the clone snippet, or remove if the surrounding sentence becomes redundant.
4. **Hard "do not touch" list**:
   - `contracts@v1` plan-entry schema id and any code constructing it.
   - `.specify/contracts/` baseline path.
   - `contracts/` change-local artifact dir.
   - `contracts.*` validation rule ids.
   - Merge behaviour for contract artifacts.
   - Workspace distribution of central contracts.
   - `rules_for("contracts")` function name and dispatch.
5. **Run tests**:

    ```bash
    cd /Users/andrewweston/rust/github.com/augentic/specify-cli
    cargo test
    ```

   Re-baseline any fixture transcripts that exercise renamed slash commands.

**Acceptance**:

- `rg -n "/plan:sow-writer|/contracts:writer|/contracts:validator|/contracts:importer|/rt:git-cloner" -- ` in `specify-cli` returns no matches.
- `cargo test` passes in `specify-cli`.

**Dependencies**: Chunks 5, 6, 7–13 in the `specify` repo (so the new directives reference real plugins/skills).

---

### Chunk 22 — Marketplace bump and changelog

**RFC**: §Migration plan, step 12

**Scope**: Bump the marketplace version and document the renames/splits/removals in the changelog.

**Edits**:

1. **`.cursor-plugin/marketplace.json`** — increment `metadata.version` (e.g. `0.24.3` → `0.25.0` since this is a breaking namespace change). Update `metadata.description` if the original mentioned old plugin names.
2. **Each plugin's `plugins/<plugin>/.cursor-plugin/plugin.json`** — bump `version` to match the marketplace bump.
3. **`docs/explanation/whats-new.md`** — add a section documenting:
   - **Renamed**:
     - `/plan:sow-writer` → `/client:sow-writer`
     - Plugin `contracts` → `interfaces`
   - **Split**:
     - `/contracts:writer`, `/contracts:validator`, `/contracts:importer` → `/interfaces:openapi`, `/interfaces:asyncapi`, `/interfaces:json-schema` (each new skill handles author / import / verify intents internally)
   - **Removed**:
     - `/rt:git-cloner` (deleted; replaced by an inlined `git clone` snippet at the two callers)
   - Note that persisted artifact paths (`.specify/contracts/`, schema id `contracts@v1`, validation rule ids `contracts.*`) are unchanged.
4. **`VERSION`** — update if the convention is to track marketplace version (it currently reads `0.24.3`).

**Acceptance**:

- `marketplace.json` version is bumped.
- All `plugins/*/.cursor-plugin/plugin.json` `version` fields match.
- `docs/explanation/whats-new.md` documents the changes.
- `make checks` passes.

**Dependencies**: Chunks 1–21 complete and passing.

---

## Validation runbook

After landing each chunk:

1. From the `specify` repo root: `make checks`. Must exit 0.
2. After Chunk 19: also run `make dev-plugins` and verify Cursor reloads the marketplace cleanly. Then `make prod-plugins` to restore.
3. After Chunk 21: from the `specify-cli` repo root, `cargo test`.

Final cumulative checks after Chunk 22:

- `rg -n "license:" plugins/**/SKILL.md` → no matches.
- `rg -n "allowed-tools:" plugins/**/SKILL.md` → no matches.
- `rg -n "/plan:sow-writer|/contracts:writer|/contracts:validator|/contracts:importer|/rt:git-cloner" -- plugins/ docs/ schemas/ .cursor/ AGENTS.md README.md` → no matches outside `rfcs/archive/` and `rfcs/rfc-10-*.md`.
- Every SKILL.md `name:` starts with its plugin directory name + `-` and is globally unique.
- Every SKILL.md body is ≤500 lines (post-frontmatter).
- Every SKILL.md description is ≤1024 chars and contains the substring "Use when".
- `plugins/spec/references/phase-outcome-contract.md` exists and is linked from `define`, `build`, `merge`, `drop`.
- `plugins/interfaces/skills/{openapi,asyncapi,json-schema}/` exist with SKILL.md + `author.md` + `importer.md` + `verifier.md` each.
- `plugins/rt/skills/git-cloner/`, `plugins/contracts/`, `plugins/plan/` no longer exist.
- `plugins/client/skills/sow-writer/SKILL.md` exists with `name: client-sow-writer`.

## Out-of-scope reminder

Per RFC §Non-goals and §E, this work does **not** change:

- Any CLI verb behaviour or argument shape.
- Specify artifact schemas, plan schemas, brief topology, or `pipeline.*` ordering.
- The brief frontmatter shape (frontmatter does not name skills today).
- The persisted contract artifact paths, schema id `contracts@v1`, or `contracts.*` validation rule ids.
- The `omnia/code-reviewer` agent-team protocol's behaviour (Chunk 15 only splits the file under the line ceiling).
