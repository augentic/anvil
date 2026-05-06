# Generate From An Updated `design.md` Created By `/spec:define`

This is a negative or boundary test for the current pipeline. Omnia/Vectis
implementation changes do not include a `contracts` define stage. They consume
baseline contracts as context, while new or changed interface shapes are
introduced through a separate `contracts@v1` change.

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

Then update `.specify/slices/loyalty-enrollment/design.md` with more specific
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

- A plain implementation-schema `/spec:define` regeneration should not derive
  contract YAML from the updated `design.md`.
- The endpoint details must be captured in a dedicated `contracts@v1` change,
  where `/spec:define` writes interface-level specs and `/spec:build` produces
  the contract delta.
- If an implementation change needs this new API, it should depend on the
  contract change and read the merged root `contracts/` files as baseline
  context.

## Recommended Regression Path

```text
/spec:define loyalty-enrollment-interface
/spec:build loyalty-enrollment-interface
/spec:merge loyalty-enrollment-interface
```

## Expected Contract Files

During `/spec:build`, the dedicated contract change should produce these
change-local contract deltas. After merge, the same paths become root
`contracts/` baseline files.

- `contracts/http/loyalty-api.yaml`
- `contracts/schemas/loyalty-enrollment-request.yaml`
- `contracts/schemas/loyalty-enrollment.yaml`
- `contracts/schemas/error-response.yaml`
