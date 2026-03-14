# Spec-Driven Development

Specify contains specialist skills that implement spec-driven software development based on Augentic frameworks such as the Omnia runtime.

## Getting Started

### Initialize the repository

After installing the Augentic plugin marketplace in Cursor, initialize spec-driven development for your project. This will initialize Specify in your project and create the `.specify/` directory populated with the specified schema.

For example:

```text
/spec:init https://github.com/augentic/specify/omnia
```

### Create a change

Once initialized, use the default workflow to work through the process of creating, implementing, and merging a change.

```text
/spec:propose -> /spec:apply -> /spec:archive
```

Use the commands to:

- `/spec:propose "Migrate <repo-url> to Rust WASM on Omnia."`: Create artifacts.
- `/spec:apply`: Apply the change.
- `/spec:archive`: Merge specs into baseline and archive.

Other commands available:

- `/spec:abandon`: Discard a change without merging specs.
- `/spec:status`: Check artifact completion and task progress.
- `/spec:explore`: Think through ideas and investigate problems.

## Development

In a new project, add the skills and references to the project's `.cursor/skills/` directory by creating symlinks to each skill:

```bash
mkdir -p .cursor/plugins && \
for plugin in $SPECIFY_REPO/plugins/*/; do
  name=$(basename "$plugin")
  ln -sfn "$plugin" ".cursor/plugins/$name"
done
```

### Validation

Validate the repository documentation and metadata with:

```bash
make checks
```

## About

### Plugins

This repository provides specialist skills to support spec-driven software development. Skills are organised — or namespeced — by plugin.

- [**Specify**](plugins/spec/) -- Core workflow orchestration: propose, apply, archive, abandon, explore, and more.
- [**Omnia**](plugins/omnia/) -- Generate and review Rust WASM crates targeting the Omnia runtime.
- [**RT**](plugins/rt/) -- TypeScript source analysis, fixture capture, and regression testing for migrations.
- [**Plan**](plugins/plan/) -- Requirements analysis, design enrichment, and SoW generation from JIRA.

### Structure

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


## Documentation

- [Specify Artifact Guidance](plugins/references/specify.md)
- [Project Rule](.cursor/rules/project.mdc)
- [Contribution Guide](CONTRIBUTING.md)
- [Cursor Skills Documentation](https://docs.cursor.com/skills)
- [Cursor Plugin Marketplace](https://cursor.com/docs/reference/plugins)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.