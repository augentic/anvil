# Monolith discovery notes

## What this fixture pins

The invocation in [`invocation.txt`](invocation.txt) — `/change:plan traffic --source monolith=./inputs` — passes a single legacy-code input through the discovery brief, which dispatches it to [`/spec:analyze --kind legacy-code`](../../../../../../spec/skills/analyze/SKILL.md) per [`capabilities/omnia/briefs/plan/discovery.md`](../../../../../../../capabilities/omnia/briefs/plan/discovery.md). The fixture pins the byte-stable output of that round-trip: [`expected/discovery.md`](expected/discovery.md) (three capability summaries) and [`expected/plans/traffic/analyze/monolith/metadata.json`](expected/plans/traffic/analyze/monolith/metadata.json) (structural-metadata sidecar).

## Fixture-build choice: (c) purpose-built three-capability tree

The C22 brief gave three options:

- (a) Trim the C21 Omnia fixture to three capabilities by deleting `src/billing/` in place.
- (b) Copy the whole C21 tree but author a three-capability `expected/discovery.md` that hides `billing-subscription`.
- (c) Author a fresh three-capability tree here.

**Picked (c).** Rationale:

- (b) creates a documented drift trap: the inputs would produce four capabilities but the expected pins only three, so re-runs under the eventual C23 discovery brief never match.
- (a) requires mutating the C21 fixture (`capabilities/omnia/briefs/ fixtures/plan/analyze/legacy-code/`), which is explicitly out of scope per the C22 guardrails — C21 stands as the Omnia-brief-level pin for the four-capability case.
- (c) lets the two fixtures serve different layers: C21 pins the `/spec:analyze` brief output on a four-capability tree; this fixture pins the `/change:plan` discovery brief output on a three-capability tree. The three files that overlap (`register.ts`, `validation.ts`, `verify.ts`, `common/ validation.ts`) are authored verbatim from C21 so the `user-registration` and `email-verification` blocks remain byte-identical across the two fixtures.

## RFC cross-reference

The [`user-registration` block](expected/discovery.md) in [`expected/discovery.md`](expected/discovery.md) reproduces the canonical sample from [`rfc-3a-monoliths.md` §*Plan-time analysis, define-time extraction*](../../../../../../../rfcs/archive/rfc-3a-monoliths.md) byte-for-byte (same `summary`, same `sources` set in alphabetical order, same `depends-on`, same hints, same `confidence: high`). Any change that alters this block here must also update the RFC and the C21 Omnia fixture in the same commit.

## Clustering signals exercised

The three capabilities were chosen to exercise the specific clustering heuristics pinned in [`capabilities/omnia/briefs/plan/analyze.md` §*Legacy-code branch*](../../../../../../../capabilities/omnia/briefs/plan/analyze.md):

- **Import-edge clustering.** `src/users/register.ts` imports from both `./validation` (local to `src/users/`) and `../auth/verify` (cross-module). The import cone drives `user-registration` to claim all three files (`register.ts`, `users/validation.ts`, `auth/verify.ts`). `email-verification` only claims `auth/verify.ts` because `verify.ts`'s exports (`consumeVerificationToken`) have standalone entry points not reached from `register.ts`.
- **Docstring capability markers.** Each source file's header docstring names its capability in an imperative one-sentence `summary` shape (e.g. "Create new user accounts with email verification."). The analyze brief lifts these verbatim into `summary:` fields.
- **Endpoint inference.** Docstring bullets of the form `Entry point: <METHOD> <path>` or `POST /auth/verify-email — ...` feed `hints.entry_points` alphabetically. `shared-validation` has no endpoint markers and correctly omits `hints:` in the expected output.
- **External-dep inference.** `pg`, `@sendgrid/mail`, and `stripe`-less `package.json` drive the `hints.external_deps` list. (`stripe` was intentionally dropped from this fixture's `package.json` — no billing capability means no Stripe dependency.)
- **Low-level shared primitives.** `src/common/validation.ts` carries a docstring that explicitly says "No domain knowledge here — callers layer capability-specific rules on top." — the clustering heuristic interprets this as a `shared-validation` capability at `confidence: medium` (structural, not behaviour-describing).

## Stability points

- **Capability order.** Alphabetical by name: `email-verification`, `shared-validation`, `user-registration`.
- **`sources` order within each block.** Alphabetical — the `user-registration` block lists `src/auth/verify.ts` before `src/users/register.ts` before `src/users/validation.ts`.
- **`depends-on` order.** Alphabetical — `[email-verification, shared-validation]` for `user-registration`.
- **`hints.entry_points` order.** Alphabetical — `[GET /auth/verify, POST /auth/verify-email]` for `email-verification`.
- **`hints.external_deps` order.** Alphabetical kebab-case — `[postgres, sendgrid]`.
- **`<!-- source-key: monolith -->` marker.** Emitted by the `/spec:analyze` skill (not this brief) immediately before every `### <name>` heading on this invocation.
- **`metadata.json` field order.** `version`, `source_key`, `language`, `loc`, `module_count`, `top_level_modules` — per [`analyze/SKILL.md` §Structural metadata](../../../../../../spec/skills/analyze/SKILL.md).
- **`top_level_modules` order.** Alphabetical relative paths.
- **No host state.** No timestamps, environment variables, absolute paths, or run IDs in either artifact.

## Downstream consumers

- **C24** (propose brief: 1:1 capability → slice mapping): the C24 propose fixture consumes this fixture's `expected/discovery.md` and emits a `plan.yaml` with three entries:
    - `user-registration` with `scope.monolith.include: [src/auth/verify.ts, src/users/register.ts, src/users/validation.ts]` and `depends-on: [email-verification, shared-validation]`.
    - `email-verification` with `scope.monolith.include: [src/auth/verify.ts]`.
    - `shared-validation` with `scope.monolith.include: [src/common/validation.ts]`. The C24 fixture's byte-stable `plan.yaml` is derived mechanically from this fixture's capability blocks — no second clustering pass.
