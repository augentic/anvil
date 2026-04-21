<!-- source-key: monolith -->
### email-verification

```yaml
summary: Verify a newly registered account via a one-time email token.
sources:
  - src/auth/verify.ts
depends-on: [user-registration]
hints:
  entry_points: [GET /auth/verify]
  external_deps: [postgres]
confidence: high
```

<!-- source-key: monolith -->
### shared-validation

```yaml
summary: Validate common user-facing inputs (email, password, name).
sources:
  - src/users/validation.ts
depends-on: []
hints:
  external_deps: []
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
confidence: high
```
