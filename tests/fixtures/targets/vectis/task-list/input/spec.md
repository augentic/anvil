# task-list Specification

## Purpose

Cross-platform Today-view task list with completion toggling, deletion with confirmation, and an empty state.

### Requirement: Task List View

ID: REQ-001
Sources: [screens, intent]
Status: agreed

The system SHALL render every task for today in a single scrollable list, with title, optional subtitle, completion checkbox, and chevron affordance per task.

#### Scenario: Today list renders

- **WHEN** the user opens the app and the today task list is non-empty
- **THEN** the screen titled "Today" renders one row per task with the title, optional subtitle, and a completion checkbox

#### Scenario: Empty state

- **WHEN** the today task list is empty
- **THEN** the screen renders an empty-state hero with the "empty-tasks-hero" image, "No tasks yet" title, and "Tap the + button to add your first task." subtitle

### Requirement: Toggle Task Completion

ID: REQ-002
Sources: [screens, intent]
Status: agreed

The system SHALL toggle a task's completion state when the user activates its checkbox; completed tasks SHALL render with strikethrough styling on the title.

#### Scenario: Toggle completion

- **WHEN** the user activates the completion checkbox on a task
- **THEN** the task's completion state flips, the row re-renders with strikethrough on the title when completed, and the change is persisted

### Requirement: Add Task Form View

ID: REQ-003
Sources: [intent]
Status: agreed

The system SHALL present an Add Task form when the user activates the floating "Add task" action; the form SHALL accept a title and an optional subtitle and create a new task on save.

#### Scenario: Add task

- **WHEN** the user activates the floating "Add task" action
- **THEN** the app navigates to the Add Task form with an empty title field and an empty optional subtitle field

#### Scenario: Save new task

- **WHEN** the user enters a non-empty title and activates Save
- **THEN** a new task is added to the today list and the app navigates back to the Task List view

#### Scenario: Reject empty title

- **WHEN** the user activates Save without entering a title
- **THEN** the form surfaces a "Title is required" error on the title field and does not create a task

### Requirement: Delete Task

ID: REQ-004
Sources: [screens, intent]
Status: agreed

The system SHALL present a delete-confirmation dialog before removing a task and SHALL remove the task only after the user confirms.

#### Scenario: Request delete

- **WHEN** the user requests deletion of a task
- **THEN** a "Delete task?" dialog opens with "This task will be removed permanently." as the message and Cancel / Delete actions

#### Scenario: Confirm delete

- **WHEN** the user activates Delete in the confirmation dialog
- **THEN** the task is removed from the today list and the change is persisted

#### Scenario: Cancel delete

- **WHEN** the user activates Cancel in the confirmation dialog
- **THEN** the dialog closes and the task remains in the list

### Requirement: Settings Entry Point

ID: REQ-005
Sources: [screens]
Status: agreed

The system SHALL expose a settings entry point in the Task List view header that navigates to the Settings view.

#### Scenario: Open settings

- **WHEN** the user activates the settings affordance in the Task List header
- **THEN** the app navigates to the Settings view

### Requirement: Persistence

ID: REQ-006
Sources: [intent]
Status: agreed

The system SHALL persist tasks across app launches using the Key-Value adapter and SHALL synchronise with a remote task service via the HTTP adapter when reachable.

#### Scenario: Persist locally

- **WHEN** a task is added, toggled, or deleted
- **THEN** the change is written to the local KV store before the operation reports success

#### Scenario: Remote sync

- **WHEN** the app launches and the remote task service is reachable
- **THEN** the today list is reconciled with the remote service's response and conflicts resolve in favour of the most recent local change

## Error Conditions

- `network-unavailable`: surfaced when the HTTP adapter cannot reach the remote service. The app continues to operate against the local KV cache.
- `invalid-title`: surfaced on the Add Task form when the title is empty or whitespace-only.

## iOS Shell Requirements

### Requirement: Stack Navigation

ID: REQ-007
Sources: [intent]
Status: agreed

The iOS shell SHALL render every view inside a single `NavigationStack`, with the Task List as the root.

#### Scenario: Navigation stack

- **WHEN** the user navigates from Task List → Add Task or Task List → Settings
- **THEN** the destination pushes onto the navigation stack and the back affordance returns to the previous view

### Requirement: Swipe to Delete

ID: REQ-008
Sources: [intent]
Status: agreed

The iOS shell SHALL expose a swipe-to-delete gesture on each task row in the Task List that opens the same delete-confirmation dialog as the explicit delete action.

#### Scenario: Swipe to delete

- **WHEN** the user swipes a task row left and activates the revealed Delete affordance
- **THEN** the "Delete task?" dialog opens; confirming removes the task

## Android Shell Requirements

### Requirement: Single Activity

ID: REQ-009
Sources: [intent]
Status: agreed

The Android shell SHALL render every view inside a single Activity using Jetpack Compose Navigation, with the Task List as the start destination.

#### Scenario: Compose navigation

- **WHEN** the user navigates from Task List → Add Task or Task List → Settings
- **THEN** the destination is pushed via Compose Navigation and the system back action returns to the previous destination

### Requirement: Edge to Edge

ID: REQ-010
Sources: [intent]
Status: agreed

The Android shell SHALL render edge-to-edge with the status bar treated as a transparent surface and SHALL honour the system insets for the FAB placement.

#### Scenario: Edge to edge

- **WHEN** the Task List renders on a device with gesture navigation
- **THEN** the system bar overlays the header surface with the correct contrast and the FAB sits clear of the system gesture inset
