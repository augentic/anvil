import { isNonEmpty, matchesPattern } from "../common/validation";

/**
 * Validate user-registration form inputs.
 *
 * Thin wrappers over the shared validation primitives in
 * src/common/validation.ts, narrowed to the email + password rules
 * the registration flow expects.
 */

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function validateEmail(email: string): void {
  if (!isNonEmpty(email) || !matchesPattern(email, EMAIL_PATTERN)) {
    throw new Error("invalid email");
  }
}

export function validatePassword(password: string): void {
  if (!isNonEmpty(password) || password.length < 8) {
    throw new Error("password must be at least 8 characters");
  }
}
