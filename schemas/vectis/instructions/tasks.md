Create the task list that breaks down the implementation work.

Follow the task format conventions defined in the define skill for
checkbox format, grouping, ordering, and skill directive tags.

Tasks are organized by build phase, not by feature. All features in
the change share a single task list ordered:
design-system first, core second, shells last.

Each task references the single feature spec at
`specs/<feature>/spec.md`. The spec contains both core requirements
and any platform-specific requirements in dedicated sections.

## Available Skills

| Directive                      | Skill                              | When to Use                          |
| ------------------------------ | ---------------------------------- | ------------------------------------ |
| `vectis:core-writer`           | Generate or update Crux core       | Core implementation tasks            |
| `vectis:core-reviewer`         | AI code review for Crux core       | Post-implementation review of core   |
| `vectis:ios-writer`            | Generate or update iOS shell       | iOS shell implementation tasks       |
| `vectis:ios-reviewer`          | AI code review for iOS shell       | Post-implementation review of shell  |
| `vectis:design-system-writer`  | Generate design system from tokens | Design system generation tasks       |
