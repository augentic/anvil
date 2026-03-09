# Tasks: add-caching

## Repo: acme/platform

### cache-service
- [ ] Add LRU cache data structure with TTL
- [ ] Implement GET /cache/{key}
- [ ] Implement PUT /cache/{key}
- [ ] Implement DELETE /cache/{key}
- [ ] Add integration tests

### api-gateway
- [ ] Add cache-service client
- [ ] Wire cache lookup into request pipeline
- [ ] Respect Cache-Control headers
- [ ] Add integration tests
