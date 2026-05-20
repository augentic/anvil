# Default Adapter

The default adapter is the foundational Specify adapter. It carries universal codex rules that apply independently of a project's implementation domain.

Its pipeline is intentionally minimal and generic. Domain-specific generation, build, review, and adoption behavior belongs in adapters such as `omnia`, `contracts`, and `vectis`.

## Codex

Universal review rules live under [`codex/`](codex/). The migrated `UNI-*` rules keep their legacy identifiers so existing review findings remain stable while reviewer skills transition to codex resolution.
