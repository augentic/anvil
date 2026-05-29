# Add search filter

## Overview

The user-list handler accepts an optional search query and returns the matching subset.

### Requirement: User list accepts a search query

ID: REQ-001
Sources: [intent]
Status: agreed

The user-list handler accepts an optional `query` parameter and, when present, returns only the users whose display fields match the query string.
