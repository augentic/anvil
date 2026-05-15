# Idempotency notes

The legacy-code branch of `/change:analyze` is byte-deterministic. Re-running

```
/change:analyze legacy-code monolith ./inputs/monolith/ ./expected/plans/legacy-code/
```

on an unchanged `inputs/monolith/` tree MUST produce byte-identical `expected/discovery.md` and `expected/plans/legacy-code/analyze/monolith/metadata.json`.

This is the acceptance gate for propose-time slicing stability: propose (RFC-3a C24) caches its `scope.<k>.include` decisions off each capability's `sources` list, and `specify plan validate` (RFC-3a C25) diffs `metadata.json` across runs to surface drift. If either artifact drifts under unchanged inputs, both downstream consumers see spurious change signals.

## Specific stability points

- **Capability order.** Alphabetical by name: `billing-subscription`, `email-verification`, `shared-validation`, `user-registration`.
- **`sources` order inside each capability.** Alphabetical. The `user-registration` block lists `src/auth/verify.ts` before `src/users/register.ts` before `src/users/validation.ts`.
- **`depends-on` order.** Alphabetical (e.g. `[email-verification, shared-validation]` for `user-registration`).
- **`hints.entry_points` order.** Alphabetical (e.g. `[GET /auth/verify, POST /auth/verify-email]` for `email-verification`).
- **`hints.external_deps` order.** Alphabetical kebab-case (e.g. `[postgres, sendgrid]`).
- **`<!-- source-key: monolith -->` marker.** Emitted by the SKILL (not this brief) immediately before every `### <name>` heading on this invocation.
- **`metadata.json` field order.** `version`, `source_key`, `language`, `loc`, `module_count`, `top_level_modules` — the shape pinned in `analyze/SKILL.md` §*Structural metadata*.
- **`top_level_modules` order.** Alphabetical relative paths.
- **No host state.** No timestamps, environment variables, absolute paths, or run IDs in either artifact.

## RFC cross-reference

The `user-registration` block in `expected/discovery.md` reproduces the canonical sample from `rfc-3a-monoliths.md` §*Plan-time analysis, define-time extraction* (same `summary`, same source set, same `depends-on`, same hints, same `confidence: high`), rendered in the on-disk shape (`### <name>` + fenced YAML) with `sources` in the canonical alphabetical order the output contract requires. Any change that alters this block in the fixture also needs a coordinated RFC update.
