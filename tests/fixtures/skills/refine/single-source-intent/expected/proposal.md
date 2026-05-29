# Add search filter

## Why

The operator asked for a search filter on the user list so an operator can narrow the rendered users to those whose fields match a query string.

## Units

- user-list — user-list handler accepting an optional search query and returning the matching subset

## Non-goals

- Server-side full-text search infrastructure.
- Saved or persisted searches.
- Search analytics or query telemetry beyond the standard handler metrics.
