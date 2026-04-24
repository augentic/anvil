# Emitted `specify plan create` invocations — monolith

The three commands the propose brief shells out when every slice is accepted without edit. Emit order is dependency-order + within- layer alphabetical (see [`schemas/omnia/briefs/plan/propose.md` §Emit order](../../../../../../../../schemas/omnia/briefs/plan/propose.md)):

1. `email-verification` — leaf (no `--depends-on`), description carries path hints from the capability's `sources:` list.
2. `shared-validation` — leaf (no `--depends-on`), description carries path hints.
3. `user-registration` — layer 1 (two `--depends-on` edges to the leaves above), description carries path hints and delta-targeting intent.

```text
specify plan create email-verification \
    --sources monolith \
    --description "Verify a newly registered account via a one-time email token. Focus on src/auth/verify.ts."
```

```text
specify plan create shared-validation \
    --sources monolith \
    --description "Validate common user-facing inputs with reusable primitives. Focus on src/common/validation.ts."
```

```text
specify plan create user-registration \
    --sources monolith \
    --depends-on email-verification --depends-on shared-validation \
    --description "Create new user accounts with email verification. Focus on src/auth/verify.ts, src/users/register.ts, src/users/validation.ts. Delta-targets email-verification and shared-validation."
```

The resulting `plan.yaml` is pinned byte-for-byte in [`plan.yaml`](plan.yaml).
