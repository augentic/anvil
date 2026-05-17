# Discovery — greenfield-app

## Capability inventory

<!-- source-key: arch-docs -->
### user-authentication

```yaml
summary: Handle user authentication and session management.
sources:
  - arch-docs
depends-on: []
hints:
  entry_points: [POST /auth/login, POST /auth/register]
  external_deps: [postgres, redis]
confidence: high
```

## Candidate inventory

<!-- source-key: arch-docs -->
### user-authentication

```yaml
kind: candidate
sources: [arch-docs]
surfaces:
  - arch-docs:http-route-auth-login
  - arch-docs:http-route-auth-register
declared-at:
  - arch-docs:architecture.md#user-authentication
```
