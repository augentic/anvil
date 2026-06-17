# Spec heading and preamble.

### Requirement: Password reset request

ID: REQ-001
Sources: [product-notes]
Status: agreed

The system lets a registered user request a password reset link by email.

### Requirement: Password reset expiry [divergence]

ID: REQ-002
Sources: [product-notes, legacy-monolith]
Status: divergence

The system expires password reset links after 30 minutes. (from product-notes)

Note: legacy-monolith observed a 24-hour expiry; documentation authority wins.
