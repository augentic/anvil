# iOS Assembly Template Manifest

Source-of-truth mapping for the chunk-3b templates. The Rust template engine
arriving in chunk 7 reads this list (or an equivalent embedded copy) to know
which template file goes to which on-disk path, and which placeholders /
capability markers it must process.

## Path mapping

Source filenames are flat -- no nested directories under `templates/vectis/ios/`.
Nested target paths (especially the `iOS/__APP_NAME__/...` segment) are produced
by the template engine, never by the on-disk layout of the templates directory.
This keeps `include_str!` paths short and matches the convention established
in `templates/vectis/core/MANIFEST.md`.

| Source (this dir)         | Target (rendered project)                              |
| ------------------------- | ------------------------------------------------------ |
| `project.yml`             | `iOS/project.yml`                                      |
| `Makefile`                | `iOS/Makefile`                                         |
| `App.swift`               | `iOS/__APP_NAME__/__APP_NAME__App.swift`               |
| `Core.swift`              | `iOS/__APP_NAME__/Core.swift`                          |
| `ContentView.swift`       | `iOS/__APP_NAME__/ContentView.swift`                   |
| `LoadingScreen.swift`     | `iOS/__APP_NAME__/Views/LoadingScreen.swift`           |
| `HomeScreen.swift`        | `iOS/__APP_NAME__/Views/HomeScreen.swift`              |

Total: 7 files (matches RFC § File Manifests § iOS Assembly).

The `__APP_NAME__` segment in target paths is substituted by the engine when
writing each file, the same as inside file contents. The substitution applies
to both directory and file-name positions (e.g. `__APP_NAME__App.swift` becomes
`CounterApp.swift`).

## Placeholder reference

Always present in the iOS templates:

| Placeholder           | Example value | Files                                                                 |
| --------------------- | ------------- | --------------------------------------------------------------------- |
| `__APP_NAME__`        | `Counter`     | `project.yml`, `Makefile`, `App.swift`, `Core.swift`, `ContentView.swift`, `HomeScreen.swift` (and the file/folder paths in MANIFEST) |
| `__APP_NAME_LOWER__`  | `counter`     | `project.yml` (bundle id prefix and per-config bundle ids)            |

`__APP_NAME_LOWER__` is the lowercase form of the app name (no other
transformations -- `TodoApp` → `todoapp`). The engine in chunk 7 derives it
from `--app-name` rather than asking the user to provide it; it never appears
on the CLI surface.

There are no capability-version placeholders in the iOS assembly today. The
shell depends only on the generated `Shared` and `SharedTypes` Swift packages,
which are produced from the core's pinned Crux versions.

## Cap-marker reference

Capability-conditional regions follow the same convention as core (paired
`<<<CAP:<name>` / `CAP:<name>>>>` lines, each on their own line). The engine
treats the entire region (markers and content inclusive) as removable when the
cap is not selected, and drops only the marker lines (preserving content) when
the cap is selected.

| Cap        | Files                  |
| ---------- | ---------------------- |
| `http`     | `Core.swift`           |
| `kv`       | `Core.swift`           |
| `time`     | `Core.swift`           |
| `platform` | `Core.swift`           |

Notes for chunk 7:

- Marker open/close lines do not nest. Every `<<<CAP:foo` must be paired with
  the next `CAP:foo>>>` on its own line.
- The Swift compiler enforces exhaustive switches on enums. Each cap-conditional
  region in `Core.swift` must include both the matching `case` arm in
  `processEffect(_:)` _and_ any helper functions it relies on, all inside the
  same CAP marker. The engine does not do dead-code elimination on Swift -- if
  the cap is selected, both the case arm and the helper land in the rendered
  file together; if not, both vanish.
- The `sse` cap intentionally has no entry in `Core.swift` today. The render-
  only baseline of `app.rs` does not declare an `Effect::Sse(...)` variant
  (see `templates/vectis/core/MANIFEST.md`'s "Notes for chunk 5/6"), so the
  Swift `Effect` enum produced by the codegen has no `.sse` case to handle.
  When chunk 6 decides whether to add the Rust-side variant, this manifest and
  `Core.swift` should grow a matching `<<<CAP:sse` block.

## Design system / Inject

The chunk-3b templates deliberately omit `VectisDesign` and `Inject` from the
project's SPM dependency list, even though the `ios-writer` reference docs
mention both. Reasons:

- **`VectisDesign`** lives at `design-system/ios/` and is produced by the
  separate `design-system-writer` skill. It is not guaranteed to exist when
  `vectis init` runs, and the iOS reference docs explicitly say "If the design
  system files do not exist, generate views without design system imports."
  The CLI's job is to produce a baseline that always compiles; the writer
  skills layer in `VectisDesign` during Update Mode when they detect it.
- **`Inject`** (hot-reload) is a per-developer convenience that requires
  network resolution at first build and an external `InjectionIII` macOS app.
  Including it would make the deterministic baseline depend on network
  connectivity for the first `xcodegen`/`xcodebuild` cycle, which violates
  the "one command, working project" promise.

If a future RFC wants either back, they can be added as cap-style toggles
(e.g. `--design-system`, `--hot-reload`) and gated by their own markers.

## Self-check

This manifest must list every file in `templates/vectis/ios/`. CI can enforce
this trivially -- restrict the awk match to backtick-wrapped tokens that look
like file names so cap names (`http`, `kv`, ...) from the cap-marker table
don't pollute the comparison:

```bash
diff \
  <(command ls -1 templates/vectis/ios | grep -v '^MANIFEST.md$' | sort) \
  <(awk -F'`' '/^\| `[A-Za-z][A-Za-z._-]*`/ { print $2 }' templates/vectis/ios/MANIFEST.md \
      | grep -E '\.[A-Za-z]+$|^Makefile$' | sort -u)
```

Run the diff after adding or renaming a template file.
