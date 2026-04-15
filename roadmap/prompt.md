I am working on horizon 2 in the roadmap — YAML configuration. I need a ssystem of configuration that is simple, flexible, deterministic, allows for system extensibility, and will work for horizon 3's multi-repo support.

Below are outlined the layers I can see required to make the configuration system work.

## Layer 1 - Repo-specific configuration.
This layer allows each repo to configure the part of the ssystem particular to its needs. Something along the lines of:

```yaml
# config.yaml (per repo)

schema: https://github.com/augentic/specify/schemas/omnia
platform: Realtime
domain: |
  Traffic-related services such as roadworks...

rules:
  proposal: |
    # proposal override rules
  specs: |
    # spec override rules
  design: |
    # design override rules
  tasks: |
    # task override rules
```

## Layer 2 - Tec stack specific configuration
This is the current schema.yaml but significantly simplified. For example:

```yaml
# schema.yaml (per technology stack)

name: omnia

pipeline:
  define:
    proposal: blueprints/proposal.md
    spec: blueprints/spec.md
    design: blueprints/design.md
    tasks: blueprints/tasks.md

  build:
    build: blueprints/build.md

  merge:
    merge: blueprints/merge.md
```

## Layer 3 - Platform-specific configuration
This layer contains a registry of all repos that make up a system or platform. For example, the backend and frontend of a web application. In our experience, larger platforms can have upwards of 100 repos. 

I want you to consider what we have, what we need and then rethink the configuration schema from the ground up. Do not be constrained by what is currently there, rather design something suitable for the longer term growth of the a system to generate and maintain code based on the define-build-merge workflow of the spec skills and powered by a framework of YAML -> Skills -> CLI.

Please research broadly and think deeply. This is critical piece of work will underpin much of our woork over the next 6 months.