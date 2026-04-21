# Emitted `specify initiative create` invocations — monolith (manifest branch)

Same emit order as
[`create-invocations.md`](create-invocations.md), but when
`user-registration` is `confidence: low` and shares
`src/auth/verify.ts` with `email-verification`, Stage C emits a v1
slice manifest and a single `--scope-manifest` for `monolith`
instead of three `--scope-include` flags. The first two slices stay
glob-scoped in the same run.

1. `email-verification` — unchanged glob invocation.
2. `shared-validation` — unchanged glob invocation.
3. `user-registration` — manifest pointer; the brief writes
   `.specify/plans/traffic/slices/user-registration.yaml` before
   shelling out.

```text
specify initiative create email-verification \
    --sources monolith \
    --scope-include monolith=src/auth/verify.ts \
    --description "Verify a newly registered account via a one-time email token."
```

```text
specify initiative create shared-validation \
    --sources monolith \
    --scope-include monolith=src/common/validation.ts \
    --description "Validate common user-facing inputs with reusable primitives."
```

```text
specify initiative create user-registration \
    --sources monolith \
    --depends-on email-verification --depends-on shared-validation \
    --scope-manifest monolith=.specify/plans/traffic/slices/user-registration.yaml \
    --description "Create new user accounts with email verification."
```

Pinned plan: [`plan-manifest.yaml`](plan-manifest.yaml). Manifest
body: [`slices/user-registration.yaml`](slices/user-registration.yaml).
