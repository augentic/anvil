# Design: add-caching

## cache-service
Add a new in-memory LRU cache with TTL-based eviction.
Expose `/cache/{key}` GET/PUT/DELETE endpoints.

## api-gateway
Wire outbound calls through the cache-service before hitting
upstream backends. Respect `Cache-Control` headers.
