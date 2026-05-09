# Iteration report template

Output format for `2f. Produce iteration report`. Use the full template on the first iteration; on subsequent iterations report only new findings and note the iteration number.

````
## Android Shell Review Report: {app-name} (iteration {N})

**Review Team**: 3 specialists + 1 antagonist
**Confidence Level**: [HIGH | MEDIUM | LOW]

### Summary
- Critical: N findings
- Warning: N findings
- Info: N findings

### Critical Findings

#### [AND-001-1] Missing screen composable for ViewModel variant
- **Rule ID**: VECTIS-003
- **File**: Android/app/src/main/java/com/vectis/{appname}/ui/screens/
- **Reviewer**: Structural Specialist
- **Antagonist**: Confirmed
- **Issue**: ViewModel variant `Settings(SettingsView)` has no corresponding
  screen composable file.
- **Fix**: Create `ui/screens/SettingsScreen.kt` and add the branch to the
  root composable.

### Warning Findings
...

### Info Findings
...

### Adversarial Review

**Antagonist Activity Summary**:

| Action       | Count   |
| ------------ | ------- |
| Confirmed    | [count] |
| Downgraded   | [count] |
| Upgraded     | [count] |
| Disputed     | [count] |
| New Findings | [count] |

**Acceptance Rate**: [confirmed / total specialist findings]%

#### Downgraded Findings
- [ID] ORIG -> NEW: rationale

#### Upgraded Findings
- [ID] ORIG -> NEW: rationale

#### Disputed Findings
- [ID] Reported as SEVERITY: "description"
  Dispute: rationale
  Lead Decision: [Included | Excluded]

#### New Findings (Missed by Specialists)
- [NEW-1] SEVERITY: description (file:line)
  Evidence: details
````

Classify each finding as **mechanical** (auto-fixable) or **design-level**.
