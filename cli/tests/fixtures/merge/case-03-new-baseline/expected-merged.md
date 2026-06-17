### Requirement: Handle OAuth callback

ID: REQ-001

The service accepts an authorization code at the callback endpoint and exchanges it for tokens.

#### Scenario: Valid code

- GIVEN a code returned by the identity provider
- WHEN the callback endpoint receives it
- THEN tokens are persisted against the user


### Requirement: Refresh expired tokens

ID: REQ-002

Refresh access tokens before expiry.

#### Scenario: Near-expiry token

- GIVEN a token within 60s of expiry
- WHEN a request is made
- THEN the refresh flow runs transparently

