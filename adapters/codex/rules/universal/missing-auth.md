---
id: UNI-021
title: Missing Authentication or Authorization Checks
severity: critical
trigger: A handler accesses sensitive data or mutates protected state without verifying identity and permissions.
---

## Rule

Authenticate callers and authorize the requested action before exposing sensitive data or changing protected state. Server-side or trusted-boundary checks must not rely solely on client-supplied claims.

## Look For

- Handler entry points that accept and act on requests without checking for an authentication token, session, or identity credential.
- Endpoints that return sensitive data, such as PII, financial records, or internal system state, without verifying the caller has read access.
- State-mutating operations such as create, update, or delete that do not verify the caller has write permission for the target resource.
- Authorization checks that rely solely on client-supplied role or permission claims without server-side verification.
- Inconsistent enforcement: some handlers in a module check auth while others in the same module do not, suggesting an oversight.

## Spec Guidance

When the spec does not define which operations require authentication or what authorization model applies, propose adding access control requirements before fixing the code.
