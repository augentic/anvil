## ADDED Requirements

### Requirement: Handle OAuth callback

ID: REQ-001

Exchange an authorization code for tokens at the callback endpoint.

#### Scenario: Valid code

- GIVEN a code from the identity provider
- WHEN the callback endpoint receives it
- THEN tokens are persisted against the user
