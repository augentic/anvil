# Design

## Context

New `login` crate implementing REQ-001.

## Domain Model

A `Session` value captures an authenticated user identity.

## Business Logic

Process username/password inputs and emit a session token (see REQ-001).

## Dependencies

None beyond the standard Omnia SDK.
