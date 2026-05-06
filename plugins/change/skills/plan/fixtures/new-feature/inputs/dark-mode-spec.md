# Dark mode

Add a dark-mode theme to the storefront. The user picks a theme in the
mobile app's settings screen; the choice persists to the backend so it
survives logouts and is consistent across devices.

## Backend (omnia-backend)

- Persist a per-user `theme` setting (`light` | `dark` | `system`).
- Expose a small HTTP API: `GET /v1/users/me/theme` and `PUT /v1/users/me/theme`.
- Default to `system` for users who have never set the preference.

## Mobile (vectis-mobile)

- Settings screen exposes a three-way picker (Light / Dark / Match
  system) bound to the new API.
- Every screen honours the active theme via the design system tokens.
- Cache the active preference locally so cold-launch renders correctly
  before the API call returns.

## Boundary

The two sides talk through a shared HTTP contract for the theme-preference
endpoints. The contract change lands first; the implementation changes
depend on it.
