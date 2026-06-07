# Account

The account service stores per-user identity, credential, and notification preferences.

Acceptance:
- Email is the unique handle; case-insensitive comparisons.
- Passwords are stored hashed (Argon2id) and never logged.

Decision: account records are soft-deleted; hard-delete is a separate operator workflow.
