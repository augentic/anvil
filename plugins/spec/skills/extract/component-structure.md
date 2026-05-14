# Extract Step 1 — Identify Component Structure

The extract SKILL.md keeps Step 1 to a one-line pointer; this file owns the
full THINK / ANALYZE / VERIFY procedure and the dependency version-pinning
table.

## THINK

Before analyzing code, reason through these questions:

1. What source language is this? (Check file extensions: .ts, .js, .go, .py,
   .rs, .java, .cs)
2. What is the entry point? (Look for: main.\*, index.\*, handler exports,
   main functions)
3. How is the code organized? (Monolithic file? Multiple modules? Layered
   architecture?)
4. What external libraries are used? (Check manifest: package.json, go.mod,
   requirements.txt, Cargo.toml)
5. What async patterns are present? (async/await, Promises, goroutines,
   callbacks, futures)
6. What types are defined? (interfaces, classes, structs, enums)
7. Is there a guest/entry-point layer? (Middleware, CORS, error mapping, body
   injection, parameter sourcing)

## ANALYZE

Read the source at `$SOURCE_PATH` and identify:

1. **Source language** — detect from file extensions.
2. Entry points (`main.*`, `index.*`, handler exports, `func main()`,
   `if __name__ == "__main__"`, etc.).
3. Module organization and file structure.
4. External dependencies from manifest files (`package.json`, `go.mod`,
   `requirements.txt`, `Cargo.toml`, `pom.xml`, etc.).
5. Async boundaries (async/await, Promises, goroutines, threads, futures,
   etc.).
6. Type definitions (interfaces, types, classes, structs, enums).
7. **Guest/entry-point layer** — middleware (CORS, auth), error code → HTTP
   status mapping, body injection/transformation, parameter sourcing, and any
   validation performed before the domain handler.

Scope filters never hide manifest files from this step — see
[scope-filters.md](scope-filters.md) §"Sentinels always read". Language
detection and dependency extraction always run against the full set of
sentinel files regardless of `$INCLUDE` / `$EXCLUDE` / `$MANIFEST`.

## Dependency version pinning

Dependency version drift is a leading cause of build failures when
regenerating from a specification. Capture dependency versions from the
source project's **lock file**, not just the manifest.

| Stack | Manifest | Lock File | Version Source |
|-------|----------|-----------|----------------|
| Rust | `Cargo.toml` | `Cargo.lock` | Lock file |
| Node/TypeScript | `package.json` | `package-lock.json` / `yarn.lock` / `pnpm-lock.yaml` | Lock file |
| Python | `pyproject.toml` / `setup.cfg` | `poetry.lock` / `requirements.txt` (pinned) | Lock file or pinned requirements |
| C# | `.csproj` | `packages.lock.json` | Lock file |
| Go | `go.mod` | `go.sum` | `go.mod` (already pinned) |
| Java/Kotlin | `pom.xml` / `build.gradle` | Dependency tree output | Resolved dependency tree |

For each dependency, record: package name, **exact version** from lock file
(e.g., `1.4.0`, not `^1.4`), whether it is direct or transitive, and any
feature flags / optional features enabled.

In the design.md Dependencies section, list the **manifest version
specifier** (e.g., `"1.0.100"` from Cargo.toml, `"^2.3.0"` from package.json)
as the primary version — this is what goes into the generated project's
dependency declaration. Also note the lock file resolved version for API
compatibility reference.

**When the lock file is absent**: use the manifest version constraints and
flag this in Risks / Open Questions.

## VERIFY

- [ ] I've identified the primary source language correctly
- [ ] I've found all entry points (there may be multiple)
- [ ] I've understood the module structure (not just listed files)
- [ ] I've checked the manifest file for dependencies
- [ ] I've noted async vs sync execution patterns
- [ ] I've checked for a guest/entry-point layer (middleware, error mapping,
      body injection)
- [ ] I've read the lock file for dependency versions (or flagged its
      absence)
