# Emitted `specify initiative create` invocations — monolith (description-driven)

Same emit order as [`create-invocations.md`](create-invocations.md). All scope and delta-targeting intent is carried in the `description` field. The define skill infers extract filters and baseline targets from the description at execution time.

1. `email-verification` — description carries path hints.
2. `shared-validation` — description carries path hints.
3. `user-registration` — description carries path hints and delta-targeting intent for both leaves.

```text
specify initiative create email-verification \
    --sources monolith \
    --description "Verify a newly registered account via a one-time email token. Focus on src/auth/verify.ts."
```

```text
specify initiative create shared-validation \
    --sources monolith \
    --description "Validate common user-facing inputs with reusable primitives. Focus on src/common/validation.ts."
```

```text
specify initiative create user-registration \
    --sources monolith \
    --depends-on email-verification --depends-on shared-validation \
    --description "Create new user accounts with email verification. Focus on src/auth/verify.ts, src/users/register.ts, src/users/validation.ts. Delta-targets email-verification and shared-validation."
```
