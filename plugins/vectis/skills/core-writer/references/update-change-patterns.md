# Update change patterns

Common change patterns and which code elements they touch. Use as a checklist when applying changes in steps U5--U7.

## Adding a view

1. Add a new variant to `enum Page`.
2. Add a corresponding variant to `enum ViewModel`, wrapping a new per-page view struct if the view carries data.
3. Define the per-page view struct with `Facet, Serialize, Deserialize, Clone, Debug, Default` derives.
4. If the view is user-navigable (not an internal state like Loading/Error), add a variant to `enum Route` and a match arm in the `Event::Navigate(route)` handler.
5. Add a match arm in `view()` that maps the new `Page` variant to the new `ViewModel` variant.
6. Add page transition logic in the relevant `update()` arms (`model.page = Page::NewView`).
7. If the view has user interactions, add shell-facing Event variants for them.

## Removing a view

1. Remove all `update()` arms that transition to the removed `Page` variant.
2. If the view had a `Route` variant, remove it from `enum Route` and from the `Navigate` match arm.
3. Remove the match arm from `view()`.
4. Remove the `Page` variant.
5. Remove the `ViewModel` variant and its per-page view struct.
6. Remove any Event variants that were exclusive to the removed view.

## Adding a feature

1. Add a new shell-facing Event variant to `enum Event`.
2. Add a match arm in `update()` with the handler logic.
3. If the feature needs new state, add a field to `Model` (with a `Default` value).
4. If the feature produces new display data, add a field to the relevant per-page view struct and update the corresponding match arm in `view()`.
5. Ensure the new Event variant is testable (test-writer will generate tests).

## Removing a feature

1. Remove the Event variant from `enum Event`.
2. Remove the match arm from `update()`.
3. Remove any Model fields that are now unused (not referenced by any remaining event handler or `view()`).
4. Remove any per-page view struct fields that are now unused.
5. Check for helper functions that are now unused and remove them.

## Modifying a feature

1. Update the Event variant payload if the signature changed.
2. Update the match arm logic in `update()`.
3. Update any Model or ViewModel fields affected by the slice.
4. Ensure modified Event variant remains testable (test-writer will update tests).

## Adding a capability

1. Add the crate to `[workspace.dependencies]` in the workspace `Cargo.toml`.
2. Add the crate to `[dependencies]` in `shared/Cargo.toml`.
3. Add a variant to `enum Effect` for the new capability's operation type.
4. Add a type alias: `type X = crate_name::X<Effect, Event>;`.
5. Add `use` imports for the capability's types.
6. If the capability is custom (not a published crate), create the module file, add `pub mod {name};` to `lib.rs`, and add `use crate::{name}::...;` to `app.rs`.
7. Add internal Event variants for the capability's callbacks (with `#[serde(skip)]` and `#[facet(skip)]`).
8. Add match arms in `update()` for the new internal Event variants.

## Removing a capability

Reverse of adding -- remove in this order to avoid compilation errors:

1. Remove match arms for the capability's internal Event variants from `update()`.
2. Remove the internal Event variants from `enum Event`.
3. Remove the type alias.
4. Remove the Effect variant.
5. Remove `use` imports.
6. Remove the crate from `shared/Cargo.toml` and workspace `Cargo.toml`.
7. If custom, delete the module file and remove `pub mod {name};` from `lib.rs`.

## Changing an API endpoint

1. Update the URL pattern in the HTTP call site within `update()`.
2. Update the HTTP method if it changed (e.g., `Http::post` -> `Http::put`).
3. Update request body structs if the payload shape changed.
4. Update response handling if the response shape changed (may require updating the internal Event variant's payload type).
5. Ensure changed API shapes remain testable (test-writer will update tests).

## Changing a business rule

1. Locate the match arm(s) in `update()` that enforce the rule.
2. Update the guard condition, validation logic, or helper function.
3. Ensure changed rule is testable (test-writer will update tests).
