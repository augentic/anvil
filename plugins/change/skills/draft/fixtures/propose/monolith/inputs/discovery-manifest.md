# Discovery — traffic

## Adapter inventory

<!-- source-key: monolith -->
### email-verification

```yaml
summary: Verify a newly registered account via a one-time email token.
sources:
  - src/auth/verify.ts
depends-on: []
hints:
  entry_points: [GET /auth/verify, POST /auth/verify-email]
  external_deps: [postgres, sendgrid]
confidence: high
```

<!-- source-key: monolith -->
### shared-validation

```yaml
summary: Validate common user-facing inputs with reusable primitives.
sources:
  - src/common/validation.ts
depends-on: []
confidence: medium
```

<!-- source-key: monolith -->
### user-registration

```yaml
summary: Create new user accounts with email verification.
sources:
  - src/auth/verify.ts
  - src/users/register.ts
  - src/users/validation.ts
depends-on: [email-verification, shared-validation]
hints:
  entry_points: [POST /users]
  external_deps: [postgres, sendgrid]
confidence: low
```
