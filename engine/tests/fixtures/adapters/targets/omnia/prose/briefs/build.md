---
id: build
description: Build brief — guides Rust crate generation and test scaffolding.
---

The Omnia target's `build` brief drives crate generation, test-suite
scaffolding, and guest-wrapper synthesis off the slice's refine-time
artifacts. The specialist `omnia` plugin skills implement the actual
work; this brief documents the contract the build operation honours.
