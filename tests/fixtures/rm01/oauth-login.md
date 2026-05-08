# OAuth Login

Customers want to sign into the Shop apps with Google or Apple instead of
creating another password. The change introduces third-party OAuth login for
the backend and mobile apps while leaving web sign-in and account merging out
of scope.

The backend needs a shared HTTP API to start the provider flow, exchange an
authorization code for a Shop session, refresh that session, and sign out.
Mobile clients need login screens, redirect handling, and background refresh
so users do not see a sign-in screen mid-session.

The API should use standard OAuth 2.0 / OpenID Connect authorization-code
flows. Mobile apps must never store provider refresh tokens. App-visible errors
must distinguish user cancellation, provider rejection, and expired Shop
sessions.
