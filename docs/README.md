# Specify Developer Guide -- Local Development

This directory contains the [mdbook](https://rust-lang.github.io/mdBook/) source for the Specify Developer Guide.

## Prerequisites

```bash
cargo install mdbook
cargo install mdbook-d2 --locked
```

You also need [D2](https://d2lang.com/) installed and available on your `PATH`:

```bash
curl -fsSL https://d2lang.com/install.sh | sh -s --
```

## Serve locally (with live-reload)

```bash
mdbook serve docs   # from the repo root
```

Opens at <http://localhost:3000> by default.

## One-off build

```bash
mdbook build docs   # from the repo root
```

Output lands in `docs/book/`.
