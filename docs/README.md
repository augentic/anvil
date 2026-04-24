# Specify Operator Guide — Local Development

This directory contains the [mdbook](https://rust-lang.github.io/mdBook/) source for the Specify Operator Guide.

## Prerequisites

```bash
cargo install mdbook
cargo install mdbook-mermaid
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
