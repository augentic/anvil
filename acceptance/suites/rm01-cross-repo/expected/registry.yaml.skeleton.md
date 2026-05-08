# Expected `registry.yaml` Shape

> Comment-annotated skeleton of the hub's `registry.yaml` after C07 setup
> primitives have run. Used by the `rm01-cross-repo` suite to assert
> registry shape **before** `/change:plan` is invoked. Owner:
> [`scenario.md`](../scenario.md) (Workspace + Inputs sections).

The registry is **always** populated through the
[`specify`](../../../../AGENTS.md) CLI (`specify init --hub`,
`specify registry add`). The runner never hand-edits this file; the
[CLI-Authoritative Invariant](../../../README.md#cli-authoritative-invariant)
forbids it.

## Asserted Skeleton

Comments inline mark which fields the suite asserts on, which are
free-form, and which are deliberately absent.

```yaml
# `version` is a CLI-managed integer. Suite asserts the field exists and is
# an integer; it does not assert a specific value.
version: 1

# Hub identity. Asserted: `name == "shop-platform"`. Other fields are
# CLI-managed and not asserted by the suite.
hub:
  name: shop-platform

# Two project entries, in any order. Suite asserts:
# - exactly two entries,
# - one entry has `name: shop-backend` and `schema: omnia@v1`,
# - one entry has `name: shop-mobile`  and `schema: vectis@v1`,
# - both entries carry a non-empty `description` (RFC-9 §2A invariant
#   `description-missing-multi-repo`),
# - both entries carry a `url` matching `git@github.com:shop/<name>.git`
#   (the fake-GitHub remote shape from `specify-cli/tests/cross_repo.rs`).
projects:
  - name: shop-backend
    url: git@github.com:shop/shop-backend.git
    schema: omnia@v1
    description: >-
      User registration, account management, OAuth provider integration,
      token storage, and the authoritative HTTP API.
  - name: shop-mobile
    url: git@github.com:shop/shop-mobile.git
    schema: vectis@v1
    description: >-
      iOS and Android mobile clients with login screens, OAuth redirect
      handling, and token refresh flows.
```

## Field Notes

- **`schema:`** is the entry-level capability identifier (not a separate
  "schema" concept). The CLI surfaces it as `--schema` on
  `specify registry add` for historical reasons; per
  [AGENTS.md](../../../../AGENTS.md) the value is a capability id of the
  form `^[a-z][a-z0-9-]*@v\d+$`.
- **`description:`** drives the planner's project-assignment step
  (RFC-3b). The two descriptions above are the load-bearing inputs that
  let the planner route the OAuth-tokens slice to `shop-backend` and the
  OAuth-screens slice to `shop-mobile`. The suite's
  `backend-slice-routed-to-shop-backend` and
  `mobile-slice-routed-to-shop-mobile` assertions depend on these
  descriptions being distinct enough to disambiguate.
- **`url:`** uses the `git@github.com:` form so the fake SSH transport
  from `specify-cli/tests/cross_repo.rs` (or the C07 helper that ports
  it) can rewrite the operation onto the local bare remote under
  `<temp-root>/remotes/<name>.git`.

## Deliberately Absent Fields

The suite asserts that the registry does **not** carry:

- A `hub.capability:` field. Hubs do not declare a capability per RFC-9
  §1D. The hub's `project.yaml` similarly carries `hub: true` and omits
  `capability:`.
- An entry for `shop-platform` itself. The hub is not a registry entry; it
  is the host of the registry.
- Any `project:` block that lacks `description:`. The
  `description-missing-multi-repo` invariant in `specify registry validate`
  must reject such a state.

## Assertion Pointers

The C09 change should lift these as runner-checked rules. They are not
new assertion ids — they are pre-`/change:plan` setup invariants that
must hold before any `plan-*` assertion is meaningful:

- `setup-hub-project-yaml-has-hub-true-and-no-capability`,
- `setup-registry-has-two-entries`,
- `setup-registry-entries-have-non-empty-descriptions`,
- `setup-registry-validate-clean` — `specify registry validate` exits `0`.

These are reserved here so C07/C09 can adopt them verbatim without
re-inventing vocabulary.
