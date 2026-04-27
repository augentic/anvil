# User Registration Specification

## Purpose

API for registering new user accounts and managing user records.

### Requirement: Create User Account

ID: REQ-001

The system SHALL accept a `POST /users` request with a `UserRegistration` payload containing `email`, `password`, and `display_name`.

#### Scenario: Successful Registration

- **WHEN** a valid `POST /users` request is received with a unique email address
- **THEN** the system responds with `201 Created` and a `User` payload including the assigned `id`, `email`, `display_name`, and `created_at` timestamp

#### Scenario: Duplicate Email

- **WHEN** a `POST /users` request is received with an email that is already registered
- **THEN** the system responds with `409 Conflict` and an `ErrorResponse` payload with code `EMAIL_ALREADY_REGISTERED`

#### Scenario: Invalid Email Format

- **WHEN** a `POST /users` request is received with a malformed email address
- **THEN** the system responds with `400 Bad Request` and an `ErrorResponse` payload with code `INVALID_EMAIL`

### Requirement: Retrieve User

ID: REQ-002

The system SHALL accept a `GET /users/{id}` request and return the user record.

#### Scenario: User Found

- **WHEN** a `GET /users/{id}` request is received with a valid user ID
- **THEN** the system responds with `200 OK` and a `User` payload

#### Scenario: User Not Found

- **WHEN** a `GET /users/{id}` request is received with a non-existent user ID
- **THEN** the system responds with `404 Not Found` and an `ErrorResponse` payload with code `USER_NOT_FOUND`

### Requirement: Delete User

ID: REQ-003

The system SHALL accept a `DELETE /users/{id}` request and remove the user record.

#### Scenario: Successful Deletion

- **WHEN** a `DELETE /users/{id}` request is received with a valid user ID
- **THEN** the system responds with `204 No Content`

#### Scenario: User Not Found

- **WHEN** a `DELETE /users/{id}` request is received with a non-existent user ID
- **THEN** the system responds with `404 Not Found` and an `ErrorResponse` payload with code `USER_NOT_FOUND`

## Error Conditions

- `INVALID_EMAIL`: Malformed email address in registration request
- `EMAIL_ALREADY_REGISTERED`: Email already in use by another account
- `USER_NOT_FOUND`: Requested user ID does not exist
