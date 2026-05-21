# Add search filter

## Motivation

The operator asked for a search filter on the user list so an operator can narrow the rendered users to those whose fields match a query string.

## Scope

- A user-list handler accepts an optional search query and returns the subset of users that match it.
- Matching is case-insensitive substring over the user's display fields.

## Non-goals

- Server-side full-text search infrastructure.
- Saved or persisted searches.
- Search analytics or query telemetry beyond the standard handler metrics.
