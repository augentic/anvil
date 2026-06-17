# `lint/minimal` fixture

A minimal consumer-project tree exercised by
`tests/review_indexer_consumer.rs`. Files exercise every per-file
extractor that ships in `specify lint` v1 (S6):

- `doc.md` — markdown with YAML frontmatter, multiple ATX headings,
  a fenced code block that hides a fake heading and a fake link, and
  a real link that resolves against this fixture tree.
- `.specify/blob.bin` — binary blob (contains a NUL byte) under the
  `.specify/` tree so the indexer's include filter accepts it; used
  to assert the binary-file branch of the walker.
- `.specify/nonutf8.json` — invalid-UTF-8 bytes under `.specify/`;
  the walker decodes lossily with U+FFFD replacement and records
  the file as `kind: text`.

At runtime the test additionally creates four entries inside the
tempdir (none of them are committed because they need to be
controlled per-OS):

1. `.gitignore` listing `ignored.md` and matching `ignored.md` to
   prove the walker honours `.gitignore`.
2. `target/build.rs` to prove the always-ignore globs cover the
   Cargo `target/` directory.
3. `link.md` → `doc.md` (a relative symlink) to exercise the
   symlink-fact recorder without committing a symlink (committed
   symlinks are fragile across operating systems and source-control
   systems).
