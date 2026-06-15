# OAuth Login

The platform needs OAuth login so mobile customers can sign in with an
external identity provider.

## Participants

- backend: owns token exchange and session creation
- mobile: owns the sign-in screen and callback handling
- identity-provider: external OAuth provider

## Contract

Define a shared OAuth login contract before implementation begins.

HTTP endpoints:

1. POST /oauth/exchange
   - Request OAuthExchangeRequest:
     - provider: string, required, enum: apple, google
     - authorization_code: string, required
     - redirect_uri: string, required
     - code_verifier: string, required
   - 200 response OAuthSession:
     - access_token: string
     - refresh_token: string
     - expires_at: date-time
     - user_id: string
   - 400 ErrorResponse for invalid input
   - 401 ErrorResponse when the provider rejects the code

2. POST /oauth/refresh
   - Request OAuthRefreshRequest:
     - refresh_token: string, required
   - 200 response OAuthSession
   - 401 ErrorResponse when the refresh token is invalid or expired

## Backend implementation

The backend should validate requests, call the identity provider, create or
update the local user session, and return the shared response contract.

## Mobile implementation

The mobile client should present provider choices, launch the OAuth flow, handle
the callback, and call the backend exchange endpoint using the shared contract.
