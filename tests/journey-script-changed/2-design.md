# Design

## Overview

Rebuild design for the session and login surface reconciled in
`spec.md`: an email/password sign-in issuing a session token, with a
30-minute inactivity expiry per the operator directive.

## Decisions

- Session expiry is enforced server-side at 30 minutes of inactivity
  (`REQ-002`); the observed 15-minute TTL is superseded.
- The acceptance criteria for session expiry are an open gap
  (`REQ-003`) and must be authored before implementation.
