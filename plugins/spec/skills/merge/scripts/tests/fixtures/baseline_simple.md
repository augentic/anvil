# User Authentication

## Overview

Authentication requirements for the application.

### Requirement: Login

ID: REQ-001

Users must be able to log in with email and password.

#### Scenario: Valid credentials

Given a registered user
When they enter correct email and password
Then they are authenticated and redirected to the dashboard

### Requirement: Logout

ID: REQ-002

Users must be able to log out and invalidate their session.

#### Scenario: Active session

Given a logged-in user
When they click logout
Then their session is invalidated and they are redirected to the login page
