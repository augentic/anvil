# Tasks

- [ ] Reconcile the `[conflict]` on REQ-001 — confirm with the operator whether the TTL is 30 minutes (product-notes) or 60 minutes (identity-design-notes), then hand-edit `spec.md` to flip to `Status: agreed` before `/spec:build`.
- [ ] Once REQ-001 is reconciled, set the `PASSWORD_RESET_EXPIRY_MINUTES` default in the Omnia config to the agreed value.
- [ ] Add a test asserting that the configured value flows through `Config::get` into the handler's expiry computation.
- [ ] Run code review on the configuration change.
