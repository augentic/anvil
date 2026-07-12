# Tasks

- [ ] Add the `SearchQuery` newtype and typed-input validation in `ListUsers::call`.
- [ ] Extend `ListUsersRequest::handle` to filter the `TableStore` results by `SearchQuery`.
- [ ] Update or add tests covering the no-query, matching-query, and empty-match cases.
- [ ] Wire `query` into the HTTP guest route for `GET /users`.
- [ ] Run code review for the user-list crate.
