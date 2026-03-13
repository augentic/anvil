I've got myself (and the spec plugin) into a bit of a pickle so I am going to describe what I want at a high level and get you to help me design and implement the change.

Currently, the spec plugin skills use a local schema directory to resolve the schema. This is not ideal because it requires the user to clone the schema repository to their local machine.

I want to change the spec plugin to optionally use a remote schema URL instead of installing a local schema directory. This will allow the user to use the spec plugin without having to clone the schema repository to their local machine.

## Schema

A schema is a collection of files that define the artifacts (id, template filename, instruction, dependencies) and the apply instruction.

```text
schemas/<name>/
├── schema.yaml      # Artifact definitions, instructions, apply instruction
├── config.yaml      # Starter config installed by /spec:init
└── templates/       # Artifact templates
    ├── proposal.md
    ├── spec.md
    ├── design.md
    └── tasks.md
```

## Thoughts

My thinking so far is as follows.

### /spec:init <schema-url>

The schema URL will be stored in the `schema` field of the `config.yaml` file.

If the /spec:init skill is used with a schema URL as an argument, the `config.yaml` file will be downloaded from the schema described at the remote URL and written to the local file system at `.specify/config.yaml`.

The `config.yaml` file will be the only file that needs to be installed. All other /spec skills will reference the the remote schema indicated by the schema URL.


### /spec:init (no arguments)

If the /spec:init skill is used without a schema URL as an argument, the `config.yaml` file will be created in the local file system at `.specify/config.yaml` with the default schema and templates

Please review the current /spec plugin skills and the schema/ directory to get a better understanding of the current schema structure and how it is used by the /spec skills. And then propose a more well thought-through, extensible schema structure and workflow.