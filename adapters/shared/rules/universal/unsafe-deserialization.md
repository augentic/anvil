---
id: UNI-020
title: Unsafe Deserialization of Untrusted Data
severity: critical
trigger: Untrusted serialized data is decoded into types or shapes that can bypass validation, escalate privileges, or exhaust resources.
---

## Rule

Do not deserialize untrusted input directly into privileged internal domain types without validation and limits. The target type and payload bounds should prevent privilege injection, unexpected fields, polymorphic abuse, and denial of service.

## Look For

- Deserializing untrusted input directly into internal domain types that carry privilege or authorization state, such as a `User` struct with an `is_admin` field populated from an external payload.
- Accepting serialized data from an external source without schema validation, allowing unexpected fields to be injected.
- Deserializing into polymorphic or trait-object types where the concrete type is attacker-controlled.
- Large or deeply nested payloads deserialized without size limits, enabling denial of service via memory exhaustion or stack overflow.
