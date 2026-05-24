# Tasks

- [ ] Author the `identity-user-registration` Omnia crate per design.md (newtypes, request, handler, repository wrapper).
- [ ] Add tests covering the RFC-5322 accept path, the invalid-email reject path, and the table-store failure path.
- [ ] Wire the `POST /users` route into the identity-svc guest, including provider impls for `TableStore` and `Config`.
- [ ] Run code review on the new crate and guest wiring.
