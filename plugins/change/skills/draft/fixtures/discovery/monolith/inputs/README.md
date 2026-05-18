# Monolith fixture source tree

Tiny three-adapter TypeScript monolith consumed by [`../invocation.txt`](../invocation.txt). Each adapter is isolated in its own `src/` subtree so the clustering heuristic in [`/change:analyze legacy-code`](../../../../../analyze/SKILL.md) can recover the three expected boundaries cleanly.

## Layout

| Path                          | Adapter           | Signals                                                                                   |
| ----------------------------- | -------------------- | ----------------------------------------------------------------------------------------- |
| `src/users/register.ts`       | `user-registration`  | Docstring names the adapter; `POST /users` entry point; imports `validation` + `verify`. |
| `src/users/validation.ts`     | `user-registration`  | Registration-specific validation wrappers; imports `../common/validation`.                 |
| `src/auth/verify.ts`          | `email-verification` | Two entry points (`POST /auth/verify-email`, `GET /auth/verify`); `sendgrid` external dep.   |
| `src/common/validation.ts`    | `shared-validation`  | Low-level predicates, no domain knowledge — the canonical shared-primitives adapter.    |

`user-registration` straddles `src/users/` and `src/auth/verify.ts` because `register.ts` calls `sendVerificationEmail` in the verification flow; the clustering heuristic follows the import edge but still keeps `email-verification` as a separate adapter because `verify.ts`'s other exports (`consumeVerificationToken`) serve an independent entry point.
