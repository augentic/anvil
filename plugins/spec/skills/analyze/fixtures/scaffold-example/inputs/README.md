# Tiny hypothetical monolith

A three-capability TypeScript service used only to illustrate the
`/spec:analyze` output shape. The files referenced below are not
present — this fixture pins the *expected output* for a notional
input, not a runnable tree.

Capabilities the scaffold example summarises:

- `user-registration` — sign-up endpoint, validation, email-verify
  kick-off. Touches `src/auth/verify.ts`, `src/users/register.ts`,
  `src/users/validation.ts`.
- `email-verification` — consumes the verification token, flips the
  account state. Touches `src/auth/verify.ts`.
- `shared-validation` — regex / length helpers used by both of the
  above. Touches `src/users/validation.ts`.

Real per-kind fixtures with actual source trees land in RFC-3a C21
(code branch) and C18 (documentation branch).
