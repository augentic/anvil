## Lead inventory

### user-registration

- id: user-registration
- sources: [runtime]
- summary: POST /users observed in 2 captures; happy path returns 201 and publishes `user.created`, error path returns 400 with `weak-password`.
