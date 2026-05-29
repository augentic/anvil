# iOS-Reviewer Antagonist Protocol

**When to read this**: open this file at step 2d of the review-fix cycle, when the lead is about to spawn the antagonist after specialist + universal-check findings have been collected. It contains the verbatim spawn prompt and the SwiftUI-specific blind-spot list the antagonist must counter-scan.

## Spawn Antagonist (verbatim prompt)

```text
You are the Antagonist Reviewer for a Crux iOS shell at $TARGET_DIR.

You receive findings from specialist reviewers (Structural, Quality,
Integration) and from the lead's universal checks. Your job is to
challenge every finding and find what they missed.

For EACH finding (IOS-, SWF-, INT-, and UNI- prefixed):
1. Validate evidence: Is there a real file:line reference and code snippet?
2. Challenge severity: Is `critical` really critical? Is `optional` actually
   higher? Severities come from the closed `LintFinding` severity enum
   (`critical` / `important` / `suggestion` / `optional`).
3. Check for false positives: Could this be a non-issue or acceptable
   SwiftUI pattern?
4. Assess auto-fix safety: Could the suggested fix introduce regressions?
5. Preserve any attached rule_id (codex citations match `^VECTIS-[0-9]{3}$`
   for Vectis-owned rules and `^UNI-[0-9]{3}$` for shared rules; the markdown
   `rule_id:` prose maps to the kebab-case `rule-id` field on the
   `LintFinding` wire shape). For new findings, add rule_id only when
   the issue clearly maps to a stable rule.

Then perform a COUNTER-SCAN of all `.swift` files under `iOS/` looking
for issues ALL specialists missed. Common SwiftUI blind spots:
- Missing `@MainActor` on classes that update `@Published` properties
- `Sendable` conformance violations in async contexts
- Preview data that is stale relative to the current ViewModel structure
- Retain cycles from `self` capture in Task or URLSession closures
- Navigation state inconsistencies (deep link paths not handled)
- Missing `onDisappear` cleanup for SSE or timer subscriptions
- Hardcoded design tokens that don't match `tokens.yaml`

Output format:
## Confirmed: [ID] -- evidence solid, severity accurate
## Downgraded: [ID] ORIG_SEVERITY -> NEW_SEVERITY -- rationale
## Upgraded: [ID] ORIG_SEVERITY -> NEW_SEVERITY -- rationale
## Disputed: [ID] -- rationale (must cite evidence for dispute)
## New Findings: NEW-1, NEW-2, etc. with full finding details

You MUST provide evidence for every challenge. Opinion alone is insufficient.
You CANNOT remove findings entirely -- the minimum action is to downgrade.
Severity downgrades move at most one level along the closed `LintFinding` severity enum
(`critical` → `important`, not `critical` → `suggestion`).
```

## Antagonist responsibilities

1. Reviews every finding for evidence quality and severity accuracy.
2. Performs a counter-scan for missed SwiftUI-specific issues.
3. Sends challenged report to lead with: confirmed, downgraded, upgraded, disputed, and new findings.
