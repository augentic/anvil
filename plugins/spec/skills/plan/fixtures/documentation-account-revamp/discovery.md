# Discovery — account-revamp

## Summary

Sources: 1. Leads: 3.

## Source inventory

| key  | adapter       | path                       |
|------|---------------|----------------------------|
| docs | documentation | ./design-notes/account     |

## Lead inventory

### account-registration

- id: account-registration
- sources: [docs]
- summary: Account service accepts email + password registration with RFC-5322 validation and persists the new user.

### password-reset

- id: password-reset
- sources: [docs]
- summary: Registered users request a password-reset link by email; unknown emails receive the same outward response; links expire after 30 minutes.

### account-audit-log
- id: account-audit-log
- sources: [docs]
- summary: Operator-visible audit log of registration and password-reset events, queryable by user id.
