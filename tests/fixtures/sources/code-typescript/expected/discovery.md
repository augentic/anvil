# Discovery — fixture extract

> Only the `## Candidate inventory` section is asserted by the acceptance harness; the surrounding sections are owned by `/spec:plan` and are reproduced here for context. The `code-typescript` enumerate brief appends candidate blocks under `## Candidate inventory`; it never writes the heading itself.

## Summary

- Sources: 1
- Candidates: 1

## Source inventory

| key             | adapter         | path           |
| --------------- | --------------- | -------------- |
| legacy-monolith | code-typescript | ./source       |

## Candidate inventory

### user-registration

- id: user-registration
- sources: [legacy-monolith]
- summary: Registration endpoint accepting email + password with RFC-5322 validation.
