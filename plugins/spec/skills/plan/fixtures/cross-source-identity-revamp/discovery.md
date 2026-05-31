# Discovery — identity-revamp

## Summary

Sources: 2. Leads: 5.

## Source inventory

| key                  | adapter         | path                          |
|----------------------|-----------------|-------------------------------|
| identity-design-notes | documentation  | ./design-notes/identity       |
| legacy-monolith      | code-typescript | ./vendor/legacy-monolith      |

## Lead inventory

### identity-design-notes:user-registration

- lead-id: user-registration
- source-key: identity-design-notes
- summary: Registration endpoint accepting email + password with RFC-5322 validation.

### legacy-monolith:user-registration

- lead-id: user-registration
- source-key: legacy-monolith
- summary: POST /users handler validating email + password and inserting the new user record.

### identity-design-notes:password-reset

- lead-id: password-reset
- source-key: identity-design-notes
- summary: Registered users request a password-reset link by email; unknown emails receive the same outward response; links expire after 30 minutes.

### legacy-monolith:account-pwd-reset

- lead-id: account-pwd-reset
- source-key: legacy-monolith
- summary: Account password-reset handler; emits a 24-hour reset token via the transactional email service.

### legacy-monolith:identity-audit-events

- lead-id: identity-audit-events
- source-key: legacy-monolith
- summary: Audit-event emitter for identity actions (registration, login, password reset).
