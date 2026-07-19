# Contribution Guide

Augentic welcomes contributions to the Specify framework. This document covers the essentials for getting a pull request accepted. For detailed guidance on the runtime, adapters, schemas, and Cursor skill wrappers, see the [Contributing section](docs/contributing/index.md) of the Developer Guide.

## Table of Contents

- [Getting started](#getting-started)
- [Code style](#code-style)
- [Developer's Certificate of Origin](#developers-certificate-of-origin)
- [Pull request procedure](#pull-request-procedure)
- [Conduct](#conduct)

## Getting started

Unless you are fixing a known bug, we recommend discussing your change with the core team via a GitHub issue before getting started to ensure alignment with the project roadmap.

The framework lives in one repository with two surfaces:

- **Rust workspace** (`src/`, `crates/`, `examples/`) — the `specify` binary, guest orchestrations, and JSON schemas it distributes
- **Prose and Cursor wrappers** (`plugins/`, `docs/`, `.cursor-plugin/`) — ultrathin `/spec:*` skill wrappers, documentation, and the marketplace manifest

Adapters live in the sibling [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters) repository.

See the [Contributing Overview](docs/contributing/index.md) for the full repository map, development environment setup, and links to topic-specific guides. To preview working-tree `/spec:*` skills against a local CLI, see [Cursor operator plugins](docs/contributing/operator-plugins.md).

## Code style

### Rust (the workspace at the repo root)

- Format with `cargo fmt`
- Lint with `cargo clippy -- -D warnings`
- Run the full suite with `cargo make ci`

### Docs and skill wrappers

- Keep `/spec:*` skill bodies ultrathin invoke-and-relay — body style is guidance in [`docs/standards/cli-contract.md`](docs/standards/cli-contract.md)
- `cargo test -p checks` must pass before submitting a pull request. It needs only a Rust toolchain. See [Consistency Checks](docs/contributing/checks.md).
- Use kebab-case for file names, change names, and adapter identifiers
- Generation behavior belongs in guest orchestrations or adapter prompts, not in skill bodies

## Developer's Certificate of Origin

All contributions must include acceptance of the DCO:

```text
Developer Certificate of Origin
Version 1.1

Copyright (C) 2004, 2006 The Linux Foundation and its contributors.
660 York Street, Suite 102,
San Francisco, CA 94110 USA

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.


Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part
    by me, under the same open source license (unless I am
    permitted to submit under a different license), as indicated
    in the file; or

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including all
    personal information I submit with it, including my sign-off) is
    maintained indefinitely and may be redistributed consistent with
    this project or the open source license(s) involved.
```

To accept the DCO, add this line to each commit message with your name and email address (`git commit -s` will do this for you):

```text
Signed-off-by: Jane Example <jane@example.com>
```

For legal reasons, no anonymous or pseudonymous contributions are accepted.

## Pull request procedure

Pull requests should be targeted at the `main` branch. Before creating a pull request, go through this checklist:

1. Create a feature branch off of `main`.
2. [Rebase](https://git-scm.com/book/en/Git-Branching-Rebasing) your local changes against `main`.
3. Run checks: `cargo test -p checks` for boundary/authoring; `cargo make links` for the Developer Guide; `make ci` (`cargo make ci`) for the full gate.
4. Accept the Developer's Certificate of Origin on all commits (see above).

All contributions are made via pull request. All patches from all contributors get reviewed. At least one review from a maintainer is required for all patches (even patches from maintainers).

Normally, all pull requests must include tests that cover your change. For skill-wrapper or docs changes, this means `cargo test -p checks` passes (and, for wrapper changes, you have manually verified the skill in a target project). For CLI changes, add or update integration tests under `crates/*/tests/`.

## Conduct

Whether you are a regular contributor or a newcomer, we care about making this community a safe place for you and we've got your back.

- We are committed to providing a friendly, safe and welcoming environment for all, regardless of gender, sexual orientation, disability, ethnicity, religion, or similar personal characteristic.
- Be kind and courteous. There is no need to be mean or rude.
- We will exclude you from interaction if you insult, demean or harass anyone. In particular, we do not tolerate behavior that excludes people in socially marginalized groups.
- Private harassment is also unacceptable. If you feel you have been or are being harassed or made uncomfortable by a community member, please contact a member of the core team immediately.

We welcome discussion about creating a welcoming, safe, and productive environment for the community. If you have any questions, feedback, or concerns please let us know with a GitHub issue.
