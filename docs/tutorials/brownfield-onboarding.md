# Brownfield Onboarding

If you have an existing codebase, you do not need to start from scratch. Specify can extract behavioral specs and a design document from your source code, giving you a baseline that reflects what the system already does.

**Prerequisites:** [Cursor IDE, Augentic plugins, and the `specify` CLI installed](../orientation/prerequisites.md). An existing codebase to onboard.

## 1. Initialise the project

Start by initialising Specify in your existing project:

```text
/spec:init https://github.com/augentic/specify/adapters/omnia
```

When init detects existing source code, it will offer to create an `initial-baseline` change for extraction. Accept the offer, or proceed manually with the steps below.

## 2. Extract specs from source code

Run extract against your codebase:

```text
/spec:extract . .specify/slices/initial-baseline/
```

<details>
<summary>Expected output</summary>

```text
Extracting from . ...
  Scanning source files...
  Discovered adapters: auth, payments, notifications

Generating artifacts...
  ✓ specs/auth/spec.md (5 requirements)
  ✓ specs/payments/spec.md (8 requirements)
  ✓ specs/notifications/spec.md (3 requirements)
  ✓ design.md

Extraction complete. Review artifacts, then run /spec:merge initial-baseline.
```

</details>

The agent reads your source code and produces:

- **`spec.md` files** (one per discovered adapter) -- behavioral requirements extracted from the source. Each requirement gets a stable ID and scenarios based on the code's actual behavior.
- **`design.md`** -- the technical shape: domain models, API contracts, dependencies, and business logic tagged with `[domain]`, `[infrastructure]`, `[mechanical]`, or `[unknown]`.

The extraction is **language-agnostic**. The artifacts describe *what* the code does, not how it is implemented. A TypeScript service and a Go service that do the same thing should produce equivalent specs.

### Scoping the extraction

For large codebases, you can narrow the scope:

```text
# Extract only the auth module
/spec:extract . .specify/slices/initial-baseline/ include "src/auth/**"

# Exclude test files
/spec:extract . .specify/slices/initial-baseline/ exclude "**/*test*"

# Use a manifest of specific files
/spec:extract . .specify/slices/initial-baseline/ manifest ./files-to-extract.txt
```

## 3. Review the extracted artifacts

Open the generated artifacts and review them:

- Do the specs accurately describe the system's behavior?
- Are the requirement IDs and scenarios reasonable?
- Does the design capture the key technical decisions?

You can ask the agent to refine specs, add missing requirements, or correct inaccuracies before merging.

## 4. Merge the baseline

Once you are satisfied with the extracted artifacts:

```text
/spec:merge initial-baseline
```

Your baseline now reflects the existing system:

```
.specify/specs/
├── auth/
│   └── spec.md         # extracted auth requirements
├── payments/
│   └── spec.md         # extracted payment requirements
└── notifications/
    └── spec.md         # extracted notification requirements
```

## 5. Start making changes

With a populated baseline, new changes benefit from context. When you define a slice that modifies an existing adapter, Specify reads the baseline spec and produces a delta:

```text
/spec:define "Add two-factor authentication to the auth service"
```

The agent knows the current auth requirements from the baseline. The generated delta spec will reference existing `REQ-XXX` IDs and add new ones.

## The onboarding flow

```text
/spec:init                                         # one-time setup
/spec:extract . .specify/slices/initial-baseline/  # extract from source
/spec:merge initial-baseline                        # establish baseline
    ... (normal define-build-merge from here)
```

## What you learned

- `/spec:extract` produces behavioral specs and design from existing source code.
- Extracted artifacts are language-agnostic -- they describe behavior, not implementation.
- Merging the extraction establishes a baseline that future changes build on.
- Scoping flags (`--include`, `--exclude`, `--manifest`) narrow extraction for large codebases.
- After onboarding, the normal define-build-merge workflow applies.

## Next

[A Multi-Slice Change](single-repo-change.md) -- coordinate multiple related slices with a plan.
