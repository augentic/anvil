## Lead inventory

### runtime:user-registration

- lead: user-registration
- source: runtime
- synopsis: POST /users observed in 2 captures; happy path returns 201 and publishes `user.created`, error path returns 400 with `weak-password`.
