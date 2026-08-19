# Design

## Overview

Rebuild design for the greeting surface reconciled in `spec.md`: one
static `GET /greeting` endpoint returning `'hello'`.

## Decisions

- The endpoint is static (`REQ-001`); no state or configuration is
  involved.
- The acceptance criteria are an open gap (`REQ-002`) and must be
  authored before implementation.
