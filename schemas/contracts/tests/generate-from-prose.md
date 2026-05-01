# Generate From Prose Passed To `/spec:define`

Use this prompt to test authoring JSON Schema and OpenAPI artifacts from prose
requirements.

Pipeline note:

- In the `contracts` schema, `/spec:define` creates `proposal.md`,
  `specs/**/*.md`, and `tasks.md`; contract YAML is produced during
  `/spec:build`.
- In Omnia and Vectis schemas, contract YAML is produced during `/spec:define`
  because the `contracts` brief is part of the define pipeline.

## Prompt

```text
/spec:define user-profile-api

Generate API contracts from prose.

Authorship Mode: Generate from prose
Participants:
- profile-service: producer
- mobile-app: consumer
- admin-console: consumer

Define a User Profile HTTP API.

Endpoints:
1. POST /profiles
   - Request body CreateProfileRequest:
     - user_id: string, required
     - display_name: string, required, 1-80 chars
     - timezone: string, optional, IANA timezone
   - 201 response Profile:
     - id: string
     - user_id: string
     - display_name: string
     - timezone: string|null
     - created_at: string date-time
   - 400 ErrorResponse for invalid fields
   - 409 ErrorResponse when a profile already exists for user_id

2. GET /profiles/{profile_id}
   - path parameter profile_id: string, required
   - 200 response Profile
   - 404 ErrorResponse when not found

3. PATCH /profiles/{profile_id}
   - Request body UpdateProfileRequest:
     - display_name: string, optional, 1-80 chars
     - timezone: string|null, optional
   - 200 response Profile
   - 400 ErrorResponse for invalid fields
   - 404 ErrorResponse when not found

All endpoints use application/json. ErrorResponse has code: string, message:
string, and optional details: object.
```

## Expected Contract Files

- `contracts/schemas/create-profile-request.yaml`
- `contracts/schemas/profile.yaml`
- `contracts/schemas/update-profile-request.yaml`
- `contracts/schemas/error-response.yaml`
- `contracts/http/profile-api.yaml`
