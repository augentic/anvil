# Worked example — `task-list` candidate

End-to-end illustration of `screenshots.extract` for a candidate named `task-list` bound under source-key `screens`. Two screen images:

```text
task-list-populated.png   # visible header: "Today"; rows of task items + FAB
task-list-empty.png       # same header / chrome; empty-state illustration replaces body
```

Resulting Evidence (only one task row's claims shown for brevity):

```yaml
source: screens
adapter: screenshots
authority: documentation
candidate: task-list
claims:
  - kind: region
    claim-id: task-list.header
    path: task-list-populated.png
    screen: task-list
    region: header
    title: Today
  - kind: region
    claim-id: task-list.body
    path: task-list-populated.png
    screen: task-list
    region: body
  - kind: region
    claim-id: task-list.fab
    path: task-list-populated.png
    screen: task-list
    region: fab
  - kind: region
    claim-id: task-list.states.empty
    path: task-list-empty.png
    screen: task-list
    region: states.empty
    state_when: tasks.is_empty
    state_replaces: body
  - kind: container
    claim-id: task-list.body.tasks
    path: task-list-populated.png
    screen: task-list
    region: body
    parent: task-list.body
    container: list
    each: tasks
    style: plain
  - kind: container
    claim-id: task-list.body.tasks.task-row
    path: task-list-populated.png
    screen: task-list
    region: body
    parent: task-list.body.tasks
    container: group
    direction: row
    gap: md
    padding: md
    align: center
    notes:
      candidate_component: task-row
  - kind: leaf
    claim-id: task-list.body.tasks.task-row.checkbox
    path: task-list-populated.png
    screen: task-list
    region: body
    parent: task-list.body.tasks.task-row
    leaf: checkbox
    label: Mark task complete
  - kind: leaf
    claim-id: task-list.fab.action
    path: task-list-populated.png
    screen: task-list
    region: fab
    parent: task-list.fab
    leaf: icon
    name: plus
```

A full input / output fixture for this example lives at [`tests/fixtures/sources/screenshots/task-list-two-screen/`](../../../../tests/fixtures/sources/screenshots/task-list-two-screen/) in the repo. When `screenshots.extract` runs against a *second* candidate later in the same plan (e.g. an `archive` screen sharing the same row skeleton), the brief promotes the candidate-component note to `component: task-row` per the pipeline's stage-6 ≥2-screens rule.
