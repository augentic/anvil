# Example 3: Update Existing Repository (Pull)

## Scenario

Repository already exists locally, fetch and pull latest changes

## Input

```
Repository URL: https://github.com/org/existing-project.git
Destination: ~/code
```

## Check Existing Repository

```bash
cd ~/code/existing-project
git status --porcelain  # Check for uncommitted changes
```

**Output**: Working tree is clean

## Execution

```bash
# Fetch and pull latest changes
git fetch && git pull
```

## Expected Output

```text
✓ Successfully updated repository!

Repository: existing-project
Location: ~/code/existing-project
Branch: main
Commits pulled: 5
Latest commit: ghi789 - Update dependencies
Changes: 12 files changed, 234 insertions(+), 89 deletions(-)
```

## Result

Repository updated with 5 new commits, no conflicts
