# Augentic Plugins

Specialist skills and references for Specify-driven Rust WASM delivery on Augentic's Omnia runtime.

## Workflow

This repository is designed for a human-driven Specify workflow:

```text
/spec:propose  ->  /spec:apply  ->  /spec:archive
```

The job of this repository is not to replace Specify. Its job is to supply specialist expertise:

- Omnia and WASM-aware code generation
- TypeScript source analysis
- JIRA requirements analysis
- Design enrichment
- Code review and verification guidance

## Getting Started

Install the Augentic plugin in Cursor, then initialize Specify in your project:

| Command | Description |
| ------- | ----------- |
| `/spec:init` | Initialize Specify in your project (creates `.specify/` structure) |
| `/spec:propose` "Migrate https://github.com/org/my-service to Rust WASM on Omnia." | Create artifacts |
| `/spec:apply` | Apply the change |
| `/spec:archive` | Merge specs into baseline and archive |
| `/spec:status` | Check artifact completion and task progress |
| `/spec:explore` | Think through ideas and investigate problems |

## Plugins

Four plugins provide specialist skills consumed during `/spec` work:

### Specify Plugin (`plugins/spec/`)

Core Specify workflow orchestration.

| Skill                | Primary role                                                                 |
| -------------------- | ---------------------------------------------------------------------------- |
| `init`               | Initialize Specify in a project                                              |
| `propose`            | Create a change and generate all artifacts in one step                       |
| `apply`              | Implement tasks from a Specify change                                       |
| `archive`            | Finalize and archive a completed change                                      |
| `explore`            | Thinking partner for exploring ideas, problems, and requirements             |
| `status`             | Check artifact completion, task progress, and active changes                 |

### Omnia Plugin (`plugins/omnia/`)

Code generation and review for Rust WASM on the Omnia runtime.

| Skill                | Primary role                                                                 |
| -------------------- | ---------------------------------------------------------------------------- |
| `crate-writer`       | Generate or update Rust crates from Specify artifacts                       |
| `test-writer`        | Generate or update test suites from Specify artifacts and crate code        |
| `guest-writer`       | Generate the WASM guest wrapper around domain crates                         |
| `code-reviewer`      | Review generated or updated crates for correctness and Omnia/WASM compliance |

### RT Plugin (`plugins/rt/`)

TypeScript source analysis and fixture capture for migration workflows.

| Skill                | Primary role                                                                 |
| -------------------- | ---------------------------------------------------------------------------- |
| `code-analyzer`      | Derive baseline Specify artifacts from an existing TypeScript codebase      |
| `git-cloner`         | Clone a source repository as a detached tree for analysis                    |
| `replay-writer`      | Add regression tests from captured real-world fixtures                       |
| `wiretapper`         | Capture fixture data from legacy services                                    |

### Plan Plugin (`plugins/plan/`)

Requirements analysis, design enrichment, and SoW generation.

| Skill                | Primary role                                                                 |
| -------------------- | ---------------------------------------------------------------------------- |
| `epic-analyzer`      | Derive proposal, specs, and design context from JIRA epics and stories       |
| `sow-writer`         | Translate Specify artifacts into client-facing SoW material                 |

## Repository Structure

```text
augentic-plugins/
├── .cursor/
│   └── rules/                    # Project guidance for agents
├── plugins/
│   ├── references/               # Shared references (specify.md, agent-teams.md)
│   ├── omnia/                    # Omnia code generation plugin
│   │   └── references/           # Omnia-specific references (guardrails, providers, etc.)
│   ├── spec/                     # Specify workflow plugin
│   │   └── references/           # Artifact templates and instructions
│   ├── plan/                     # Plan requirements analysis plugin
│   └── rt/                       # RT migration plugin
├── schemas/                      # Schema definitions (reference documentation)
└── scripts/                      # Documentation and consistency checks
```

## Validation

Validate the repository documentation and metadata with:

```bash
make checks
```

## Documentation

- [Specify Artifact Guidance](plugins/references/specify.md)
- [Project Rule](.cursor/rules/project.mdc)
- [Contribution Guide](CONTRIBUTING.md)
- [Cursor Skills Documentation](https://docs.cursor.com/skills)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
