/**
 * Shared validation primitives.
 *
 * Low-level predicates reused by every capability that accepts
 * user-facing input. No domain knowledge here — callers layer
 * capability-specific rules on top.
 */

export function isNonEmpty(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

export function matchesPattern(value: string, pattern: RegExp): boolean {
  return pattern.test(value);
}

export function withinRange(value: number, min: number, max: number): boolean {
  return Number.isFinite(value) && value >= min && value <= max;
}
