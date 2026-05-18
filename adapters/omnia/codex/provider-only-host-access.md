---
id: OMNIA-001
title: Provider-Only Host Access
severity: critical
trigger: An Omnia Rust crate performs external I/O, reads configuration, stores state, publishes messages, or accesses identity without going through Omnia SDK provider traits.
---

## Rule

Omnia business logic must treat host resources as provider adapters. External I/O, configuration, identity tokens, durable state, document/table/blob storage, publishing, and broadcast access must flow through `omnia_sdk` provider traits on explicit generic `provider: &P` bounds. Domain crates must not construct host clients, call raw WASI modules, open sockets or files, or hide I/O behind custom abstractions.

Function bounds should name only the provider traits the function actually calls. Handler bounds should be the union of the traits required by the handler and the helpers it invokes.

## Look For

- Direct client construction such as `reqwest::Client`, `mongodb::Client`, `RedisClient::connect`, `TableClient`, `BlobServiceClient`, `Producer::new`, or storage SDK clients.
- Forbidden storage or transport crates such as `reqwest`, `redis`, `sqlx`, `diesel`, `mongodb`, `azure_storage_blobs`, `aws-sdk-s3`, or host-side `hyper`.
- Calls to raw WASI modules or guest boundary helpers from domain crates instead of `omnia_sdk` traits.
- Functions that perform I/O but do not accept a generic `provider: &P` argument with matching trait bounds.
- Handler bounds missing a provider trait used by a helper, or blanket bounds that obscure which I/O the handler performs.
- Custom wrapper clients around provider traits that make tests mock the wrapper instead of the Omnia provider adapter.

## Spec Guidance

When artifacts describe an external dependency but not its Omnia adapter, ask for the provider mapping instead of recreating the source runtime client. Prefer the closest first-party provider: `HttpRequest`, `Config`, `Identity`, `StateStore`, `TableStore`, `DocumentStore`, `Blobstore`, `Publish`, or `Broadcast`.
