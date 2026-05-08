# OAuth Login

> Fixture feature brief for the [`rm01-cross-repo`](../../scenario.md)
> acceptance suite. This document is the **only** prose input the planner is
> allowed to read; it is intentionally user-facing and concise so
> `/change:plan` must derive the slice structure rather than parrot a
> pre-seeded plan.

## Why

Customers want to sign into the Shop apps with the identity provider they
already use (Google or Apple) instead of creating yet another password.
Today everyone has to register an email/password account, which costs us
sign-ups on first launch and pushes a meaningful number of password-reset
tickets to support.

This change introduces **third-party OAuth login** alongside the existing
email/password flow. After this lands, a new customer should be able to
finish first sign-in in under thirty seconds without ever typing a
password.

## In Scope

- Adding **Google** and **Apple** as OAuth providers across the backend
  and the iOS / Android mobile apps.
- A shared HTTP API the mobile apps and any future web client can call to
  start an OAuth flow, exchange a provider authorization code for a Shop
  session, and refresh a session as it ages.
- Persisting Shop-side identity rows for users who sign in via OAuth so
  account history (orders, addresses, loyalty) follows them across logins.
- Token-refresh behaviour on the mobile apps: the user should not be
  bounced to a sign-in screen mid-session if their access token expires
  while a screen is open.

## Out Of Scope

- Web sign-in: the web storefront keeps its current email/password flow
  for this change.
- Single sign-on between Shop properties (loyalty portal, support portal).
- Account merging between an existing email/password account and a
  newly-OAuth-linked identity. We will accept duplicate accounts in this
  first cut and address merging in a follow-up.
- Anything in the order or checkout flow.

## User Stories

- As a **first-time mobile customer**, I want to tap "Continue with
  Google" or "Continue with Apple" on the sign-in screen so I can finish
  account creation without typing a password.
- As a **returning mobile customer** who originally signed in with
  Google, I want my next launch to drop me straight into the app without
  re-prompting me for credentials, as long as my session is still valid.
- As a **returning mobile customer** whose access token has just expired,
  I want the app to refresh my session in the background so I do not see
  a sign-in screen mid-task.
- As a **support agent**, I want each Shop account record to indicate
  which provider (email, Google, or Apple) the customer used so I can
  walk them through the right reset path on a call.
- As an **account-security engineer**, I want every Shop-side identity to
  carry the provider's stable subject id (not the email) so a user who
  changes their provider email keeps the same Shop account.

## Constraints And Notes

- Use the providers' standard OAuth 2.0 / OpenID Connect authorization-code
  flows; do not invent a custom token format on the Shop side.
- The mobile clients must not see or store the provider's refresh token —
  Shop owns the long-lived session and issues its own short-lived
  access token to the apps.
- The HTTP surface should be the same shape for both providers (one
  start endpoint, one exchange endpoint, one refresh endpoint, one
  sign-out endpoint), parametrised by provider name. Adding a third
  provider later should be an HTTP-contract-only change.
- Errors visible to the apps must distinguish "the user cancelled the
  provider flow" from "the provider rejected the code" from "the Shop
  session has expired". The apps render different UI for each.
