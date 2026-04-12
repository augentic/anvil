# Delta: Improve Login

## RENAMED Requirements

## REMOVED Requirements

## MODIFIED Requirements

### Requirement: Login

ID: REQ-001

Users must be able to log in with email and password, including MFA support.

#### Scenario: Valid credentials

Given a registered user
When they enter correct email and password
Then they are prompted for MFA verification

#### Scenario: MFA verification

Given a user who passed primary authentication
When they enter a valid MFA code
Then they are authenticated and redirected to the dashboard

## ADDED Requirements
