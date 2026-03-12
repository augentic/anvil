# Proposal: add-caching

## Summary
Introduce a shared caching layer to reduce upstream latency.
The `cache-service` exposes a simple key-value HTTP API;
the `api-gateway` consults it before forwarding requests.

## Affected services
- **cache-service** — new service (LRU cache with TTL)
- **api-gateway** — updated to integrate with cache-service
