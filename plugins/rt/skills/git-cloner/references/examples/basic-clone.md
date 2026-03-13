# Example 1: Basic Clone

## Scenario

Clone a public repository to a specific directory

## Input

```
Repository URL: https://github.com/augentic/pipeline.git
Destination: ~/projects
```

## Execution

```bash
# Permission request
echo "Initializing git operations with full permissions" && date

# Clone repository
git clone --depth 1 https://github.com/augentic/pipeline.git ~/projects/pipeline
```

## Expected Output

```text
✓ Successfully cloned repository!

Repository: pipeline
Location: ~/projects/pipeline
Branch: main
Latest commit: abc123 - Add new feature
Clone type: shallow
```

## Result

Repository successfully cloned with shallow history (last commit only)
