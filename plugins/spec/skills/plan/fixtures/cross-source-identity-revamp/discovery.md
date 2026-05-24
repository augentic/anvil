# Discovery — identity-revamp

## Summary

Sources: 2. Candidates: 4.

## Source inventory

| key                  | adapter         | path                          |
|----------------------|-----------------|-------------------------------|
| identity-design-notes | documentation  | ./design-notes/identity       |
| legacy-monolith      | code-typescript | ./vendor/legacy-monolith      |

## Candidate inventory

### user-registration

- id: user-registration
- sources: [identity-design-notes, legacy-monolith]
- summary: Registration endpoint accepting email + password with RFC-5322 validation.

### password-reset

- id: password-reset
- sources: [identity-design-notes]
- summary: Registered users request a password-reset link by email; unknown emails receive the same outward response; links expire after 30 minutes.
- tentative: true

### account-pwd-reset

- id: account-pwd-reset
- sources: [legacy-monolith]
- summary: Account password-reset handler; emits a 24-hour reset token via the transactional email service.
- tentative: true

### identity-audit-events

- id: identity-audit-events
- sources: [legacy-monolith]
- summary: Audit-event emitter for identity actions (registration, login, password reset).
