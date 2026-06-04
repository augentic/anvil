# Tasks

- [ ] Author the `identity-password-reset` Omnia crate per design.md (newtypes, request, handler, repository wrapper).
- [ ] Add tests covering the known-email path, the unknown-email path, and the 30-minute expiry boundary.
- [ ] Wire the `POST /password-reset` route into the identity-svc guest, including provider impls for `TableStore`, `Publish`, and `Config`.
- [ ] Reconcile the `[divergence]` on REQ-002 (expiry) — confirm the 30-minute window with the operator before promoting the slice past `built` if the legacy 24-hour value is the correct one.
- [ ] Run code review on the new crate and guest wiring.
