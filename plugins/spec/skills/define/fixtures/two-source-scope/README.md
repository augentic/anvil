# two-source-scope — per-source scope-bundle collection on a two-source change

Pins the collection rule documented in `../../SKILL.md` §*Per-source
scope collection*. The plan entry `cross-source-refactor` declares two
sources — `monolith` (glob-filtered) and `shared-lib` (manifest-based)
— and `/spec:execute` forwards the flag surface verbatim. This fixture
shows how `/spec:define` groups the flags into per-key scope bundles
before handing each bundle to the schema's per-source extract loop,
which translates the bundle into `/spec:extract`'s native `--include`
/ `--exclude` / `--manifest` flags.
