# Tasks

- [ ] Add `SearchQuery` newtype and `from_input` parsing to the user-list crate.
- [ ] Extend `ListUsersRequest::handle` to filter the `TableStore` results by `SearchQuery`.
- [ ] Update or add tests covering the no-query, matching-query, and empty-match cases.
- [ ] Wire `query` into the HTTP guest route for `GET /users`.
- [ ] Run code review for the user-list crate.
