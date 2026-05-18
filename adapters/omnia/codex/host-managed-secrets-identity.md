---
id: SEC-001
title: Host-Managed Secrets and Identity
severity: critical
trigger: An Omnia crate embeds, derives, logs, persists, or reads credentials or tokens outside the host-managed Config and Identity provider flow.
---

## Rule

Secrets, credentials, connection strings, and access tokens must be supplied by the Omnia host and reached through approved provider traits. Source code must not contain literal secrets, read environment variables directly, construct storage authentication headers manually, derive tokens from keys, log secret values, or persist tokens in component state.

Authenticated HTTP should follow the `Config` -> `Identity::access_token` -> `HttpRequest::fetch` sequence. Storage, database, document, table, blob, and publish authentication should use the corresponding Omnia provider instead of raw signed requests or embedded credentials.

## Look For

- Literal API keys, bearer tokens, connection strings, passwords, private keys, SAS tokens, SharedKey credentials, or certificate material.
- `std::env::var`, dotenv loaders, or config files used to retrieve secrets inside a guest component.
- `Authorization` headers built from hardcoded strings, configuration values that are themselves tokens, or manually generated HMAC signatures.
- Azure Table, blob storage, SQL, Redis, MongoDB, or message-broker access implemented with raw credentials instead of `DocumentStore`, `TableStore`, `Blobstore`, `StateStore`, `Publish`, or `Identity`.
- `tracing`, `println!`, debug output, error descriptions, or test snapshots that include secret values or bearer tokens.
- Tokens cached in `static`, `OnceCell`, `StateStore`, request structs, or serialized output.

## Spec Guidance

Specs and designs should name the required config keys, identity names, and provider adapters, not secret values. If a source migration exposes an environment variable containing a secret, capture the dependency as host-managed configuration or identity and keep the value out of artifacts.
