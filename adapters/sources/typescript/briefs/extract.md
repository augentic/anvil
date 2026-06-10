# TypeScript / JavaScript source extract

`/spec:refine` invokes this brief once per `slices[].sources[]` binding whose adapter is `typescript`. Your job: for a single `(source, lead)` pair, locate the matching TypeScript module(s) under `$SOURCE_DIR`, read the surrounding code, and emit one Evidence YAML document the CLI persists to `.specify/slices/<slice>/evidence/<source>.yaml`.

## Inputs

- **`$SOURCE_DIR`** — read-only preopen of the operator-bound source root (same path the survey brief walked). Walk it; resolve `tsconfig.json` `paths` mappings relative to it.
- **`<lead>`** — the kebab-case id of the `## Lead inventory` block the slice is bound to. Look it up in `discovery.md` (the runner provides it via the binding); the block tells you which surface(s) to extract.
- **`<source>`** — the kebab-case source key the binding resolves through.

`$PROJECT_DIR` is unreachable; do not attempt to read project lifecycle state. Writes back into `$SOURCE_DIR` are denied. Use `$SCRATCH_DIR` for any internal staging.

## Reference shelf

Load on demand when the lead's surface needs deeper analysis. The bodies carry TypeScript-specific extraction depth.

- [`references/business-logic.md`](../references/business-logic.md) — depth-first domain extraction by handler / module.
- [`references/component-structure.md`](../references/component-structure.md) — language detection, entry points, module organisation, async patterns.
- [`references/dependencies.md`](../references/dependencies.md) — external service classification (database, message broker, cache, identity provider, API, WebSocket).
- [`references/external-api.md`](../references/external-api.md) — tracing deserialization code for HTTP/API calls; URLs, headers, request/response shapes, auth, retries, timeouts.
- [`references/observability.md`](../references/observability.md) — metric and trace capture: names, types, emission points, labels.
- [`references/scope-filters.md`](../references/scope-filters.md) — include / exclude / manifest filter semantics.
- [`references/verification.md`](../references/verification.md) — final validation checklist before emitting evidence.
- [`references/design-template.md`](../references/design-template.md) — 14-section design.md template (used by downstream synthesis).
- [`references/language-mapping.md`](../references/language-mapping.md) — TypeScript → Rust mapping cheatsheet (idioms, error handling, async, serialization).
- [`references/context-gaps.md`](../references/context-gaps.md) — strategies for inferring missing context when source is incomplete.
- [`references/lessons-learned.md`](../references/lessons-learned.md) — empirical wisdom from past extraction passes.
- [`references/semantic-search.md`](../references/semantic-search.md) — codebase search strategies for finding behaviour.
- [`references/examples/`](../references/examples/) — worked examples: outbound HTTP, branching/caching, parallel execution.

## Output: Evidence YAML

Return one Evidence document matching `schemas/evidence.schema.json`. The CLI atomically writes it to `evidence/<source>.yaml`; you produce the body. Top-level fields are required:

```yaml
authority: behaviour
lead: <lead>
claims:
  - kind: excerpt
    path: <ts-path>#L<start>-L<end>
    excerpt: "<short context — see Anchors and excerpts>"
  - kind: type
    path: <ts-path>#L<line>
    signature: "<type alias / interface / class signature>"
  - kind: call
    path: <ts-path>#L<line>
    callee: "<module>:<symbol>"
```

`authority` is fixed at `behaviour` for this adapter. `lead` is kebab-case (validated by `evidence.schema.json` against `^[a-z0-9]+(-[a-z0-9]+)*$`). The document's `(slice, source)` identity is path-borne (the CLI persists it at `.specify/slices/<slice>/evidence/<source>.yaml`) and the adapter resolves from `plan.yaml.sources.<source>.adapter`, so neither is written in-document. `claims: []` is valid when the lead has no in-scope code under `$SOURCE_DIR` — failure surfaces as a host-runner error, not as an empty file.

## Claim kinds

This adapter emits three kinds from the closed enum (`evidence.schema.json#/$defs/claimKind`):

- **`excerpt`** — a behavioural code span. Use this for handler bodies, validation logic, error paths, and other behaviour the requirement / criterion synthesis will reconcile on. One claim per span; spans should be focused (typically 5–80 lines of source) and accompanied by a short `excerpt:` field carrying enough context for the reader to understand the behaviour. **Do not dump raw file contents.** The `path:` anchor is the source of truth; the `excerpt:` field is short context, not a verbatim file paste.
- **`type`** — a declared interface, type alias, class declaration, or DTO. Use this when synthesis will need the shape of an input / output (e.g. `CreateUserDto`, `RegistrationResult`). The body field is `signature:` — the declaration's source spelling (one line preferred; multi-line acceptable for short class headers).
- **`call`** — an observed cross-module call that contributes to the lead's behaviour. Use this when synthesis must know that a handler delegates to another module (the call is the wire). The body field is `callee:` — `<module>:<symbol>` matching the `handler` resolution rules from the survey brief (named export, `<ClassName>.<method>`, framework-suffixed inline arrow, etc.).

`id` is optional on `excerpt` / `type` / `call` (per `evidence.schema.json` — required only on `requirement` and `criterion`). You MAY carry it for deterministic cross-source reconciliation when the claim corresponds to a stable concept; otherwise omit it.

## Anchors and excerpts

Every claim's `path:` carries a `<path>` or `<path>#L<n>` or `<path>#L<start>-L<end>` anchor matching the `evidence.schema.json` claim-path grammar (`^[^\s][^\s]*(#L[1-9][0-9]*(-L[1-9][0-9]*)?)?$`). Paths are relative under `$SOURCE_DIR` (no leading `/`, no `..`, not under a skip-root). The anchor IS the citation; the body field carries short context.

Rules for the body fields:

- **No raw file dumps.** Anchors point at the source; the YAML must not paraphrase or restate large spans. Keep `excerpt:` to a paragraph or so of focused context (the validation rule, the error response, the side effect) — never tens of lines of `"\n"`-separated source.
- **One claim per concept.** Two overlapping excerpts of the same handler are noise; pick the smallest range that captures the behaviour.
- **Stable spans across reruns.** Choose anchors at named-function or block boundaries when possible so re-extraction produces byte-stable Evidence even when surrounding lines shift slightly.
- **Symbols, not phrasing.** `call.callee` is `<file>:<symbol>` matching the survey brief's handler resolution; not free-form prose. `type.signature` is the declaration's source spelling.

## Worked example

Bound lead `user-registration` against a small Express service at `$SOURCE_DIR` (the source tree from the survey brief's worked example, source key `legacy-monolith`).

Source files in scope (per the lead's surface in the staged JSON):

- `src/server.ts` — `app.post("/users", registerUser)` at L5.
- `src/users/register.ts` — `registerUser` handler with email validation at L12–L34 and a delegation to `insertUser`.
- `src/users/repository.ts` — `insertUser` declaration plus the `User` interface.

Resulting Evidence YAML:

```yaml
authority: behaviour
lead: user-registration
claims:
  - kind: excerpt
    path: src/users/register.ts#L12-L34
    excerpt: "Handler validates email against RFC-5322 regex, returns 400 with `{ error: \"invalid-email\" }` on failure, otherwise inserts the user and returns 201 with the persisted record."
  - kind: type
    path: src/users/repository.ts#L1-L4
    signature: "interface User { id: string; email: string; createdAt: Date }"
  - kind: call
    path: src/users/register.ts#L31
    callee: "src/users/repository.ts:insertUser"
```

Three claims, three anchors, no raw source bodies. Synthesis reconciles these into `Status: agreed` requirements with `Sources: [legacy-monolith]` when no other source contributes; when documentation or intent also contributes, the authority precedence (`intent > documentation > behaviour`) defined in [`authority.md`](../references/spec-runtime/synthesis/authority.md) decides.

## Path rules

Same skip-root and traversal rules as the survey brief: relative paths only, no `..`, no leading `/`, never under `node_modules`, `vendor`, `target`, `.venv`, `dist`, `build`, no `*.d.ts` files. A symlink inside `$SOURCE_DIR` pointing outside is denied at canonicalization; the host runner returns `source-extract-path-denied` and the slice stays `refining` per workflow §Extraction reliability.

## Anti-patterns

- **Raw file dumps in `excerpt:`.** Anchors point at lines; the body field is short context, not a verbatim paste. A 200-line `excerpt:` field is wrong even when the underlying span is 200 lines.
- **Speculative claims.** Do not infer behaviour the code does not exhibit. If the handler does not enforce uniqueness, do not emit a uniqueness `excerpt`. Synthesis tags unknowns; you do not.
- **Tests-as-evidence.** Skip `*.test.*`, `*.spec.*`, `tests/`, `__tests__/`. Test files document expected behaviour; this adapter extracts observed behaviour from production source.
- **Type-only `.d.ts` files.** A `.d.ts` declares ambient types, not behaviour. Use the originating `.ts` file when possible; emit no claim when only a `.d.ts` is reachable.
- **Cross-source synthesis.** Do not reconcile this lead's claims with another source's Evidence — that is core synthesis's job in `/spec:refine` after every `extract` returns (see [From sources to slices](../references/spec-runtime/reconciliation.md#slice-time-evidence-becomes-a-spec)). Emit Evidence purely from `$SOURCE_DIR`.
- **Whole-file paths without anchors.** A `path: src/users/register.ts` claim is legal under the schema but useless for synthesis. Always anchor to the smallest meaningful range.

## Failure modes

| Condition                                                | Action                                                                                                                          |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Lead id not present in `discovery.md`               | The runner refuses to invoke the brief; not a brief-level failure mode.                                                         |
| Lead maps to no file under `$SOURCE_DIR`            | Return `claims: []`. Core synthesis surfaces `[unknown]` on every affected requirement.                                         |
| Read denied outside `$SOURCE_DIR` / `$CAPABILITY_DIR`    | Host runner returns `source-extract-path-denied`; slice stays `refining` and no Evidence is written.                            |
| Production source uses an out-of-scope framework only    | Emit any in-scope `excerpt` / `type` / `call` claims; the gap surfaces as `[unknown]` requirements at synthesis.                |
| `evidence.schema.json` validation fails on emit          | CLI rejects the Evidence; slice stays `refining`. Re-emit with the missing `id` / `kind` / `path` corrected.              |
