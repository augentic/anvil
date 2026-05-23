# Specify docs partials

This directory holds the mdbook-template partials used by chapters to compose
the visual system in [`../example.html`](../example.html). Each `.md` file is
a snippet of CommonMark + raw HTML that gets substituted in-place when a
chapter writes:

```text
{{#template ../templates/<partial>.md key=value …}}
```

Paths are resolved relative to the chapter file's location, so chapters in
`docs/` use `templates/<partial>.md` and chapters under `docs/reference/`
use `../templates/<partial>.md`. See
[`../standards/doc-authoring.md`](../standards/doc-authoring.md) for argument
contracts and copy-paste examples.

Partials with `*-open.md` / `*-close.md` pairs are wrapper blocks; everything
between the two invocations is rendered as normal markdown. Wherever a
single self-closing partial exists, the body (if any) is passed via a named
argument such as `caption=`.

This directory is **not** linked from `SUMMARY.md` and is excluded from the
search index by virtue of mdbook ignoring non-summary `.md` files.
