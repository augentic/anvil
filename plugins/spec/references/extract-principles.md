# Extract principles (non-negotiable)

These nine principles govern every `/spec:extract` run. They are non-negotiable: a violation of any of them is a regression in the artifacts. The SKILL.md body keeps a one-paragraph summary; this reference carries the full list.

1. **Focus**: Extract only domain/business logic and its inputs/outputs. Exclude infrastructure unless part of a domain rule.
2. **Descriptive, not interpretive**: Produce algorithmic descriptions of what the code does. Do not infer "why" unless present in source.
3. **Zero inference**: Do not invent behavior or semantics. Use explicit `unknown` tokens.
4. **Explicit constants**: List every constant by identifier and semantic availability.
5. **Traceability**: Each statement must be traceable to code. Do not attribute intent not in comments.
6. **Tagging**: Each Business Logic line must include one tag: `[domain]`, `[infrastructure]`, `[mechanical]`, or `[unknown]`.
7. **Conservatism**: Prefer `unknown` over guessing.
8. **Language-agnostic**: Do not introduce target-language concepts. Describe behavior, not implementation.
9. **Depth-first when possible**: When the source has clear functional domain boundaries, analyze depth-first by domain (all types + handlers + utilities for one domain before moving to the next). Fall back to step-by-step for simpler or single-domain components.
