# Discovery — account-revamp

## Summary

Sources: 1. Leads: 3.

## Source inventory

| key  | adapter       | path                       |
|------|---------------|----------------------------|
| docs | documentation | ./design-notes/account     |

## Lead inventory

### docs:account-registration

- lead: account-registration
- source: docs
- summary: Account service accepts email + password registration with RFC-5322 validation and persists the new user.

### docs:password-reset

- lead: password-reset
- source: docs
- summary: Registered users request a password-reset link by email; unknown emails receive the same outward response; links expire after 30 minutes.

### docs:account-audit-log

- lead: account-audit-log
- source: docs
- summary: Operator-visible audit log of registration and password-reset events, queryable by user id.
