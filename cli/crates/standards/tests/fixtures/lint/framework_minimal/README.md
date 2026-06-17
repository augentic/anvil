# framework_minimal

Tiny framework-repo fixture for the `scan_profile: framework`
indexer test. Each extractor (`skill`, `adapter`, `marketplace`,
`brief`) is exercised by at least one file in this tree. The
`agent-teams.md` symlink is **minted at test time** rather than
committed because relative symlinks survive `git` poorly across
operating systems; see `tests/lint_framework_indexer.rs`.

The fixture deliberately omits any top-level README or extension that
the framework include set would skip, so every fact in the model
corresponds to a deliberately authored file.
