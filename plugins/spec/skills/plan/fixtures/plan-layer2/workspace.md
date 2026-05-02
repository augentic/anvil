# Workspace — example-initiative

## alpha

- **Slot:** `.specify/workspace/alpha/`
- **Description:** Alpha project for frontend UI and user-facing features.
- **Schema:** `vectis@v1`
- **Materialisation:** symlink
- **Head:** —
- **Dirty:** —
- **Specify tree:**
  - `plan.yaml`

## beta

- **Slot:** `.specify/workspace/beta/`
- **Description:** Beta project for backend API and business logic.
- **Schema:** `omnia@v1`
- **Materialisation:** git-clone
- **Head:** `a1b2c3d4e5f6789012345678901234567890abcd`
- **Dirty:** no
- **Specify tree:**
  - `plan.yaml`
  - `changes/checkout/`
  - `specs/auth/spec.md`
