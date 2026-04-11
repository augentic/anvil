# Example 2: Clone Specific Branch with Full History

## Scenario

Clone a specific branch with complete git history for development work

## Input

```
Repository URL: https://github.com/user/backend-service.git
Branch: develop
Destination: ./workspace
Clone type: full
```

## Execution

```bash
# Clone with specific branch and full history
git clone -b develop https://github.com/user/backend-service.git ./workspace/backend-service
```

## Expected Output

```text
✓ Successfully cloned repository!

Repository: backend-service
Location: ./workspace/backend-service
Branch: develop
Latest commit: def456 - Fix API endpoint
Clone type: full
Repository size: 125 MB
```

## Result

Full repository history cloned on develop branch, ready for development
