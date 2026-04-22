# Emitted `specify initiative create` invocations — monolith

The three commands the propose brief shells out when every slice
is accepted without edit. Emit order is dependency-order + within-
layer alphabetical (see
[`schemas/omnia/briefs/plan/propose.md` §Emit order](../../../../../../../../schemas/omnia/briefs/plan/propose.md)):

1. `email-verification` — leaf (no `--depends-on`), single scope
   hint lifted verbatim from the capability's `sources:`.
2. `shared-validation` — leaf (no `--depends-on`), single scope
   hint lifted verbatim.
3. `user-registration` — layer 1 (two `--depends-on` edges to the
   leaves above), three scope hints lifted verbatim, one of which
   (`src/auth/verify.ts`) overlaps `email-verification`'s scope
   and is expected to trip a `scope-overlap` warning at validate
   time.

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
    --scope-include monolith=src/auth/verify.ts \
    --scope-include monolith=src/users/register.ts \
    --scope-include monolith=src/users/validation.ts \
    --description "Create new user accounts with email verification."
```

The resulting `plan.yaml` is pinned byte-for-byte in
[`plan.yaml`](plan.yaml).
