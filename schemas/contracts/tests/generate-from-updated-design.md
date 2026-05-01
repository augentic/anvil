# Generate From An Updated `design.md` Created By `/spec:define`

This is a negative or boundary test for the current pipeline. In Omnia/Vectis,
the define order is `specs -> contracts -> design`, so a `design.md` produced or
edited later in the same define run is not an upstream source for contract
generation.

## Initial Prompt

Start with a high-level change:

```text
/spec:define loyalty-enrollment

Create a loyalty enrollment capability. It should expose an HTTP API, but leave
endpoint details initially high level:
- customers can enroll in loyalty
- duplicate enrollment is rejected
- enrollment returns a loyalty account identifier
```

## Updated Design Detail

Then update `.specify/changes/loyalty-enrollment/design.md` with more specific
contract detail:

```markdown
## API Contracts

POST /loyalty/enrollments

Request LoyaltyEnrollmentRequest:
- customer_id: string, required
- email: string, required, format email
- referral_code: string, optional

Responses:
- 201 LoyaltyEnrollment with id, customer_id, tier, created_at
- 400 ErrorResponse for invalid email
- 409 ErrorResponse when customer_id is already enrolled
```

## Expected Behavior

- A plain regeneration of the `contracts` artifact should not reliably derive
  contract YAML from the updated `design.md`.
- The endpoint details must be reflected in `specs/**/*.md`, or the pipeline
  must be changed so the contracts brief depends on `design`.

## Recommended Regression Path

```text
/spec:define loyalty-enrollment specs
/spec:define loyalty-enrollment contracts
```

## Expected Contract Files

After the spec carries the endpoint detail:

- `contracts/http/loyalty-api.yaml`
- `contracts/schemas/loyalty-enrollment-request.yaml`
- `contracts/schemas/loyalty-enrollment.yaml`
- `contracts/schemas/error-response.yaml`
