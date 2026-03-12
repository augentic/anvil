# cache-api spec

## ADDED: GET /cache/{key}
Returns the cached value or 404.

## ADDED: PUT /cache/{key}
Stores a value with optional TTL header.

## ADDED: DELETE /cache/{key}
Evicts a key from the cache.
