# Identity password reset expiry

## Overview

The password-reset token TTL is governed by a single operator-confirmed value.

### Requirement: Reset link expiry [conflict]

ID: REQ-001
Sources: [identity-design-notes, product-notes]
Status: conflict

Note: product-notes says reset links expire after 30 minutes.
Note: identity-design-notes says reset links expire after 60 minutes.

Operator reconciliation required before /spec:build.
