---
id: UNI-018
title: Hardcoded Secrets and Credentials
severity: critical
trigger: Source code contains literal credentials, tokens, keys, connection strings, or private key material.
---

## Rule

Do not embed secrets or credentials directly in source code. Secrets grant access to protected resources and should be supplied through approved secret management or configuration mechanisms rather than committed literals.

## Look For

- String literals matching known secret prefixes such as `sk-`, `pk-`, `ghp_`, `Bearer `, `AKIA`, or `xox-`.
- Variables or constants named `password`, `secret`, `token`, `api_key`, `apikey`, `auth`, or `credential` assigned a literal value.
- Base64-encoded strings longer than 20 characters in `const` or `static` declarations.
- URLs containing embedded credentials, such as `https://user:pass@...`.
- Private keys or certificates inlined as string literals.

## Exemption

Placeholder or example values such as `https://api.example.com`, `your-api-key-here`, and test fixtures with obviously fake tokens are acceptable.
