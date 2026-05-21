---
id: extract
description: Extract behavioural Evidence (`excerpt` / `type` / `call` claims) for one candidate from the TypeScript source tree bound at $SOURCE_DIR.
authority: behaviour
---

# TypeScript / JavaScript source extract

`/spec:refine` invokes this brief once per `slices[].sources[]` binding whose adapter is `code-typescript`. Your job: for a single `(source-key, candidate-id)` pair, locate the matching TypeScript module(s) under `$SOURCE_DIR`, read the surrounding code, and emit one Evidence YAML document the CLI persists to `.specify/slices/<slice>/evidence/<source-key>.yaml`.

## Inputs

- **`$SOURCE_DIR`** — read-only preopen of the operator-bound source root (same path the enumerate brief walked). Walk it; resolve `tsconfig.json` `paths` mappings relative to it.
- **`<candidate-id>`** — the kebab-case id of the `## Candidate inventory` block the slice is bound to. Look it up in `discovery.md` (the runner provides it via the binding); the block tells you which surface(s) to extract.
- **`<source-key>`** — the kebab-case source key the binding resolves through.

`$PROJECT_DIR` is unreachable; do not attempt to read project lifecycle state. Writes back into `$SOURCE_DIR` are denied. Use `$SCRATCH_DIR` for any internal staging.

## Output: Evidence YAML

Return one Evidence document matching `schemas/evidence.schema.json`. The CLI atomically writes it to `evidence/<source-key>.yaml`; you produce the body. Top-level fields are required:

```yaml
source: <source-key>
adapter: code-typescript
authority: behaviour
candidate: <candidate-id>
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

`authority` is fixed at `behaviour` for this adapter. `source`, `adapter`, and `candidate` are kebab-case (validated by `evidence.schema.json` against `^[a-z0-9]+(-[a-z0-9]+)*$`). `claims: []` is valid when the candidate has no in-scope code under `$SOURCE_DIR` — failure surfaces as a host-runner error, not as an empty file.

## Claim kinds

This adapter emits three kinds from the closed enum (`evidence.schema.json#/$defs/claimKind`):

- **`excerpt`** — a behavioural code span. Use this for handler bodies, validation logic, error paths, and other behaviour the requirement / criterion synthesis will fuse on. One claim per span; spans should be focused (typically 5–80 lines of source) and accompanied by a short `excerpt:` field carrying enough context for the reader to understand the behaviour. **Do not dump raw file contents.** The `path:` anchor is the source of truth; the `excerpt:` field is short context, not a verbatim file paste.
- **`type`** — a declared interface, type alias, class declaration, or DTO. Use this when synthesis will need the shape of an input / output (e.g. `CreateUserDto`, `RegistrationResult`). The body field is `signature:` — the declaration's source spelling (one line preferred; multi-line acceptable for short class headers).
- **`call`** — an observed cross-module call that contributes to the candidate's behaviour. Use this when synthesis must know that a handler delegates to another module (the call is the wire). The body field is `callee:` — `<module>:<symbol>` matching the `handler` resolution rules from the enumerate brief (named export, `<ClassName>.<method>`, framework-suffixed inline arrow, etc.).

`claim-id` is optional on `excerpt` / `type` / `call` (per `evidence.schema.json` — required only on `requirement` and `criterion`). You MAY carry it for deterministic cross-source fusion when the claim corresponds to a stable concept; otherwise omit it.

## Anchors and excerpts

Every claim's `path:` carries a `<path>` or `<path>#L<n>` or `<path>#L<start>-L<end>` anchor matching the `evidence.schema.json` claim-path grammar (`^[^\s][^\s]*(#L[1-9][0-9]*(-L[1-9][0-9]*)?)?$`). Paths are relative under `$SOURCE_DIR` (no leading `/`, no `..`, not under a skip-root). The anchor IS the citation; the body field carries short context.

Rules for the body fields:

- **No raw file dumps.** Anchors point at the source; the YAML must not paraphrase or restate large spans. Keep `excerpt:` to a paragraph or so of focused context (the validation rule, the error response, the side effect) — never tens of lines of `"\n"`-separated source.
- **One claim per concept.** Two overlapping excerpts of the same handler are noise; pick the smallest range that captures the behaviour.
- **Stable spans across reruns.** Choose anchors at named-function or block boundaries when possible so re-extraction produces byte-stable Evidence even when surrounding lines shift slightly.
- **Symbols, not phrasing.** `call.callee` is `<file>:<symbol>` matching the enumerate brief's handler resolution; not free-form prose. `type.signature` is the declaration's source spelling.

## Worked example

Bound candidate `user-registration` against a small Express service at `$SOURCE_DIR` (the source tree from the enumerate brief's worked example, source key `legacy-monolith`).

Source files in scope (per the candidate's surface in the staged JSON):

- `src/server.ts` — `app.post("/users", registerUser)` at L5.
- `src/users/register.ts` — `registerUser` handler with email validation at L12–L34 and a delegation to `insertUser`.
- `src/users/repository.ts` — `insertUser` declaration plus the `User` interface.

Resulting Evidence YAML:

```yaml
source: legacy-monolith
adapter: code-typescript
authority: behaviour
candidate: user-registration
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

Three claims, three anchors, no raw source bodies. Synthesis fuses these into `Status: agreed` requirements with `Sources: [legacy-monolith]` when no other source contributes; when documentation or intent also contributes, the authority hierarchy (`intent > documentation > behaviour`) decides.

## Path rules

Same skip-root and traversal rules as the enumerate brief: relative paths only, no `..`, no leading `/`, never under `node_modules`, `vendor`, `target`, `.venv`, `dist`, `build`, no `*.d.ts` files. A symlink inside `$SOURCE_DIR` pointing outside is denied at canonicalization; the host runner returns `source-extract-path-denied` and the slice stays `refining` per RFC-25 §Extraction reliability.

## Anti-patterns

- **Raw file dumps in `excerpt:`.** Anchors point at lines; the body field is short context, not a verbatim paste. A 200-line `excerpt:` field is wrong even when the underlying span is 200 lines.
- **Speculative claims.** Do not infer behaviour the code does not exhibit. If the handler does not enforce uniqueness, do not emit a uniqueness `excerpt`. Synthesis tags unknowns; you do not.
- **Tests-as-evidence.** Skip `*.test.*`, `*.spec.*`, `tests/`, `__tests__/`. Test files document expected behaviour; this adapter extracts observed behaviour from production source.
- **Type-only `.d.ts` files.** A `.d.ts` declares ambient types, not behaviour. Use the originating `.ts` file when possible; emit no claim when only a `.d.ts` is reachable.
- **Cross-source synthesis.** Do not fuse this candidate's claims with another source's Evidence — that is core synthesis's job in `/spec:refine` after every `extract` returns. Emit Evidence purely from `$SOURCE_DIR`.
- **Whole-file paths without anchors.** A `path: src/users/register.ts` claim is legal under the schema but useless for synthesis. Always anchor to the smallest meaningful range.

## Failure modes

| Condition                                                | Action                                                                                                                          |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Candidate id not present in `discovery.md`               | The runner refuses to invoke the brief; not a brief-level failure mode.                                                         |
| Candidate maps to no file under `$SOURCE_DIR`            | Return `claims: []`. Core synthesis surfaces `[unknown]` on every affected requirement.                                         |
| Read denied outside `$SOURCE_DIR` / `$CAPABILITY_DIR`    | Host runner returns `source-extract-path-denied`; slice stays `refining` and no Evidence is written.                            |
| Production source uses an out-of-scope framework only    | Emit any in-scope `excerpt` / `type` / `call` claims; the gap surfaces as `[unknown]` requirements at synthesis.                |
| `evidence.schema.json` validation fails on emit          | CLI rejects the Evidence; slice stays `refining`. Re-emit with the missing `claim-id` / `kind` / `path` corrected.              |
