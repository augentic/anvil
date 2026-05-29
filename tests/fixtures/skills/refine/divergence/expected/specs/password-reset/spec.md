# Identity password reset

## Overview

The password-reset endpoint accepts a registered user's email, persists a reset token, dispatches a reset email, and returns the same outward response in both the known-email and unknown-email branches.

### Requirement: Password reset request

ID: REQ-001
Sources: [identity-design-notes, legacy-monolith]
Status: agreed

The account service lets a registered user request a password reset link by email; the handler returns the same outward response regardless of whether the email is known, so unknown emails cannot be enumerated.

### Requirement: Reset link expiry [divergence]

ID: REQ-002
Sources: [identity-design-notes, legacy-monolith]
Status: divergence

The system expires password reset links after 30 minutes. (from identity-design-notes; documentation)

Note: legacy-monolith observed a 24-hour expiry computed from the persisted token row's `createdAt` plus a 24h TTL constant; the documentation authority overrides. Operator review recommended.
