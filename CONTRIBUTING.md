# Contribution Guide

Augentic welcomes contributions to the Specify framework. This document covers the essentials for getting a pull request accepted. For detailed guidance on the framework's internals -- skills, schemas, plugins, and CLI architecture -- see the [Contributing section](docs/contributing/index.md) of the Developer Guide.

## Table of Contents

- [Getting started](#getting-started)
- [Code style](#code-style)
- [Developer's Certificate of Origin](#developers-certificate-of-origin)
- [Pull request procedure](#pull-request-procedure)
- [Conduct](#conduct)

## Getting started

Unless you are fixing a known bug, we recommend discussing your change with the core team via a GitHub issue before getting started to ensure alignment with the project roadmap.

The framework lives in one repository with two trees:

- **prose root** (`plugins/`, `docs/`, `.cursor-plugin/`) -- skills, briefs, shared references, and documentation (Markdown, YAML, and shell)
- **[`cli/`](cli)** -- the `specify` binary, its Rust workspace crates, and the JSON schemas it distributes

See the [Contributing Overview](docs/contributing/index.md) for the full repository map, development environment setup, and links to topic-specific guides.

## Code style

### Rust (the workspace at the repo root)

- Format with `cargo fmt`
- Lint with `cargo clippy -- -D warnings`
- Run the full suite with `cargo make ci`

### Skills and documentation (specify)

- Follow the conventions of existing `SKILL.md` files in the same plugin -- see [Skill Authoring Standards](docs/standards/skill-authoring.md)
- `cargo test --test framework` must pass before submitting a pull request. It needs only a Rust toolchain: the framework checks are plain cargo tests. See [Consistency Checks](docs/contributing/checks.md).
- Use kebab-case for file names, change names, and adapter identifiers

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
3. Run checks: `cargo test --test framework` for the prose surface; `make ci` (`cargo make ci`) for the full gate.
4. Accept the Developer's Certificate of Origin on all commits (see above).

All contributions are made via pull request. All patches from all contributors get reviewed. At least one review from a maintainer is required for all patches (even patches from maintainers).

Normally, all pull requests must include tests that cover your change. For skill changes, this means `cargo test --test framework` passes and you have manually verified the skill in a target project. For CLI changes, add or update integration tests in the `tests/` directory.

## Conduct

Whether you are a regular contributor or a newcomer, we care about making this community a safe place for you and we've got your back.

- We are committed to providing a friendly, safe and welcoming environment for all, regardless of gender, sexual orientation, disability, ethnicity, religion, or similar personal characteristic.
- Be kind and courteous. There is no need to be mean or rude.
- We will exclude you from interaction if you insult, demean or harass anyone. In particular, we do not tolerate behavior that excludes people in socially marginalized groups.
- Private harassment is also unacceptable. If you feel you have been or are being harassed or made uncomfortable by a community member, please contact a member of the core team immediately.

We welcome discussion about creating a welcoming, safe, and productive environment for the community. If you have any questions, feedback, or concerns please let us know with a GitHub issue.
