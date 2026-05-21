# discovery.md — expected `## Candidate inventory` section after `screenshots.enumerate`

> Only the `## Candidate inventory` section is owned by the source adapter brief. The CLI writes `## Summary` and `## Source inventory` from elsewhere in `/spec:plan`. This fixture shows the candidate inventory blocks the brief returns; the CLI appends them under the existing heading.

## Candidate inventory

### archive

- id: archive
- sources: [screens]
- summary: Archive: completed tasks the user has archived.

### task-list

- id: task-list
- sources: [screens]
- summary: Task list: today's open tasks for the signed-in user.
