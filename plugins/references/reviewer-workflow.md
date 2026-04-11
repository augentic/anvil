# Reviewer Workflow

Shared orchestration for Vectis reviewer skills (core-reviewer, ios-reviewer,
android-reviewer). Each skill provides platform-specific specialist prompts,
universal checks skip tables, and auto-fix lists, then follows this workflow.

See [Agent Team Patterns](agent-teams.md) for team roles, antagonist protocol,
synthesis rules, file ownership, and confidence scoring.

---

## Review-Fix Cycle (max 3 iterations)

Before starting, initialize:

- `iteration = 1`, `max_iterations = 3`
- An empty list of **accumulated design-level findings** (carried across
  iterations)

The cycle repeats: spawn the team, run specialist analysis, challenge via
antagonist, synthesize findings, auto-fix mechanical issues, then re-review
the fixes. The cycle exits when no mechanical fixes are applied or
`max_iterations` is reached.

### Team initialization

**CREATE** agent team with specialists appropriate for the current iteration
and scope. Each receives the target-dir path and their assigned review scope.

- **First iteration (`scope = full`)**: Spawn all specialists defined by
  the skill (typically Structural, Logic or Integration, and Quality).
- **First iteration (`scope = quick`)**: Spawn Structural and Quality only.
  Skip the domain-specific specialist.
- **Subsequent iterations**: Spawn only Structural and Quality, scoped to
  files modified by the previous iteration's fixes. Skip the
  domain-specific specialist -- mechanical fixes do not alter the concerns
  it checks.

### Specialist analysis (concurrent)

The specialists analyze the target concurrently. Each reads all relevant
source files but reports only on their assigned checks.

**Lead waits** for all specialists to complete before proceeding.

### Universal checks (lead; skip if scope = quick)

After all specialists report, the lead reads `universal-checks.md`
and applies checks UNI-001 through UNI-021 with platform-specific detection
heuristics. Skip checks already covered by specialists (per the skill's skip
table) and apply the remaining checks using the platform-specific heuristics
defined in the skill.

Prefix findings from this step with `UNI-`. Use the severity defined in the
universal checklist for each check.

Tag findings that have a **Spec-change indicator** (UNI-002, UNI-004,
UNI-007, UNI-008, UNI-011, UNI-012, UNI-014, UNI-021) for inclusion in the
adversarial review and spec-change output.

### Adversarial challenge

After the specialist reports and universal checks are complete, the lead
sends all combined findings to the antagonist. The antagonist:

1. Reviews every finding for evidence quality and severity accuracy
2. Performs a counter-scan for missed platform-specific issues
3. Sends challenged report to lead with: confirmed, downgraded, upgraded,
   disputed, and new findings

The antagonist spawn prompt must include:

- The target directory path
- All specialist and universal findings
- Platform-specific blind spots to counter-scan for
- The standard output format (Confirmed / Downgraded / Upgraded / Disputed /
  New Findings headings)
- Evidence requirement, no-removal rule, and one-level downgrade limit

### Synthesis

The lead merges all findings into a single iteration report:

1. **Confirmed findings**: Include verbatim from specialist reports
2. **Downgraded findings**: Include with the antagonist's revised severity
   and rationale
3. **Upgraded findings**: Include with the antagonist's revised severity
   and rationale
4. **Disputed findings**: Lead makes final call; if included, add dispute
   note
5. **New findings**: Include with the antagonist's severity and evidence
6. Assign overall **confidence level** per
   [Agent Team Patterns - Confidence Scoring](agent-teams.md#confidence-scoring)

Classify each finding as **mechanical** (auto-fixable) or **design-level**
(requires architectural decisions). Add design-level findings to the
accumulated list.

### Auto-fix mechanical issues

The **lead** applies all auto-fixes directly (specialists and antagonist have
completed their analysis). The finding prefix tracks which reviewer or pass
identified the issue.

Apply fixes for findings that are mechanical and confirmed or upgraded (not
disputed). Do NOT auto-fix structural or logic issues without explicit
confirmation -- these may require design decisions. Respect antagonist
regression flags.

After fixes, run the platform's formatter/compiler. If fixes cause errors,
revert all auto-fixes and warn in the report.

### Loop control

After applying fixes, verifying, and shutting down the team:

1. If **no mechanical fixes** were applied in this iteration, exit the cycle.
2. If `iteration >= max_iterations`, exit the cycle.
3. Otherwise, increment `iteration` and return to team initialization.

When the cycle exits, shut down all remaining teammates and output a summary
across all iterations:

```
### Review Cycle Summary
- Iteration 1: Fixed N mechanical issues (IDs).
  M design-level findings deferred. Confidence: HIGH.
- Iteration 2: Fixed K regressions from iteration 1 fixes.
  No new design-level findings. Confidence: HIGH.
- Total: N+K mechanical fixes applied. M design-level findings accumulated.
```

---

## Express Design-Level Findings as a Specify Change

After the review-fix cycle completes, check whether any **design-level
findings** were accumulated. If none were accumulated across any iteration,
skip this step.

### Classify findings: code-fix vs spec-change

- **Code-fix**: The spec is clear and the code simply does not implement it
  correctly. The fix is a code change; no spec update is needed. These
  become tasks in `tasks.md`.
- **Spec-change**: The spec is silent, ambiguous, or mandates behavior that
  the review identified as problematic. The fix requires updating the spec
  first, then implementing. These become requirements in `specs/` and
  decisions in `design.md`.

Universal checks with a Spec-change indicator (UNI-002, UNI-004, UNI-007,
UNI-008, UNI-011, UNI-012, UNI-014, UNI-021) commonly surface as spec-change
findings. Consult `universal-checks.md` for the indicator description
on each check.

### Delegate to `/spec:define`

If design-level findings exist, create a single Specify change that tracks
all of them.

1. **Derive a change name** from the app name, platform suffix, and current
   timestamp:

   ```
   review-{app-name}-{platform}-{YYYY-MM-DDTHH-MM}
   ```

   Use the shell to get the current timestamp:
   ```bash
   date -u +"%Y-%m-%dT%H-%M"
   ```

2. **Delegate to `/spec:define`** with the derived change name and a
   description synthesized from the accumulated design-level findings.

3. **Content guidelines for each artifact**:

   - **proposal.md**: The "Why" section summarizes accumulated review
     findings by severity and risk, distinguishing spec-change findings
     from code-fix findings. The "What Changes" section lists each
     design-level finding prefixed with `[spec]` or `[code]`. Note which
     mechanical fixes were already applied. The "Impact" section identifies
     affected files and contract changes.

   - **design.md**: Each design-level finding becomes a Decision section
     with rationale and alternatives. Group related findings. Reference
     the specific check IDs that motivated each decision.

   - **specs/**: Create one spec file per logical area. Each requirement
     maps to a review finding. Use WHEN/THEN format.

   - **tasks.md**: Order tasks by dependency -- spec updates first, then
     structural changes, then handler logic, then verification. Each task
     references the finding ID it addresses. Include a final verification
     task that re-runs the reviewer skill to confirm all Critical findings
     are resolved.

4. **Show final status** using `/spec:status` and prompt the user with
   "Run `/spec:build` or ask me to implement to start working on the tasks."

---

## Common Verification Checklist Items

### Team Execution

- [ ] All specialists spawned with correct category assignments
- [ ] All specialists completed before antagonist spawned
- [ ] Antagonist received all specialist + universal findings
- [ ] Antagonist provided evidence for every challenge
- [ ] Lead synthesized all findings with confidence scoring
- [ ] Team shut down and cleaned up

### Report Quality

- [ ] Each issue has file:line reference and code snippet
- [ ] Severity reflects antagonist adjustments (upgrades/downgrades applied)
- [ ] Adversarial Review section included with challenge statistics
- [ ] Confidence level assigned based on antagonist results
- [ ] Design-level findings classified as code-fix or spec-change
