---
name: git-cloner
description: Clone git repositories autonomously with validation, error handling, and flexible options.
argument-hint: [repo-url] [dest-dir] [detach?]
allowed-tools: Read, Write, StrReplace, Shell
---

# Git Cloner Skill

## Overview

Clone a git repository to a specified location with comprehensive validation, error handling, and status reporting. This skill operates **autonomously** -- it uses sensible defaults and never prompts the user for input. If required arguments are missing, it fails with a clear error message.

## Derived Arguments

1. **Repository URL** (`$REPO_URL`): HTTPS or SSH URL of the git repository (required)
2. **Destination Directory** (`$DEST_DIR`): Local path to clone into (required)
3. **Detach** (`$DETACH`, optional): If `true` or `"true"`, remove **only** the `.git` directory after clone/pull so the result is a plain tree (no git link). **Preserve** all working-tree files including `.gitignore`, `.gitattributes`, and any other ignore/attribute files — do not remove or overwrite them. Default: false.

```text
$REPO_URL = $ARGUMENTS[0]
$DEST_DIR = $ARGUMENTS[1]
$DETACH   = $ARGUMENTS[2] === true OR $ARGUMENTS[2] === "true"   # optional, default false
```

If repo-url or dest-dir is missing, fail with: `"Error: Both repo-url and dest-dir are required."`

## Process

### Step 1: Validate Inputs

1. **Validate Repository URL**:
   - Check if URL starts with `https://`, `http://`, or `git@`
   - Ensure URL is not empty or obviously malformed
   - If invalid, fail with: `"Error: Invalid repository URL: $REPO_URL"`

2. **Validate Destination Path**:
   - Ensure path doesn't contain suspicious characters
   - Check if parent directory exists or can be created
   - If invalid, fail with: `"Error: Invalid destination path: $DEST_DIR"`

3. **Verify Git Installation**:
   - Run `git --version` to confirm git is available
   - If not installed, fail with: `"Error: git is not installed"`

### Step 2: Extract Repository Name

Extract the repository name from the URL:

- Remove trailing `.git` if present
- Take the last path component
- Examples:
  - `https://github.com/augentic/pipeline.git` → `pipeline`
  - `git@github.com:user/my-repo.git` → `my-repo`
  - `https://gitlab.com/group/subgroup/project` → `project`

### Step 3: Check Existing Repository

Check if the repository already exists at `$DEST_DIR/<repo_name>`:

1. **If directory exists AND contains `.git` subdirectory**:
   - Check for uncommitted changes: `git status --porcelain`
   - If there are uncommitted changes:
     - Fail with: `"Error: Repository has uncommitted changes at $DEST_DIR/<repo_name>. Resolve manually before retrying."`
   - If clean:
     - Fetch and pull automatically: `git fetch && git pull`
     - Report number of commits pulled
   - If `$DETACH` is true, run: `rm -rf $DEST_DIR/<repo_name>/.git` only. Do not remove or alter `.gitignore`, `.gitattributes`, or other working-tree files (preserve them so ignore rules still apply if the tree is used elsewhere).
   - Skip to Step 5 (Post-Operation Verification)

2. **If directory exists but is NOT a git repository**:
   - Fail with: `"Error: Directory exists but is not a git repository: $DEST_DIR/<repo_name>"`

3. **If directory does NOT exist**:
   - Proceed to Step 4 (Clone)

### Step 4: Clone Repository

1. **Create destination directory**:

   ```bash
   mkdir -p $DEST_DIR
   ```

2. **Build clone command** using defaults:
   - Base: `git clone`
   - Shallow clone by default: `--depth 1`
   - Default branch (repository's default)
   - No submodules by default
   - Source and destination: `$REPO_URL $DEST_DIR/<repo_name>`

3. **Execute clone**:

   ```bash
   git clone --depth 1 $REPO_URL $DEST_DIR/<repo_name>
   ```

4. **Handle errors** (see Error Handling section below)

5. **If `$DETACH` is true**: Remove only the `.git` directory: `rm -rf $DEST_DIR/<repo_name>/.git`. The directory will no longer be a git repository. **Do not** remove or modify `.gitignore`, `.gitattributes`, or any other files in the working tree; preserve them.

### Step 5: Post-Operation Verification

After successful clone or pull, navigate to the repository and gather information:

```bash
cd $DEST_DIR/<repo_name>
```

- **If `$DETACH` was true**: The directory is a plain tree (no `.git`). Verify the directory exists and contains the expected source files (including `.gitignore` / `.gitattributes` if present). Skip git commands. In the summary, note "Detached: no git link (plain tree); .gitignore preserved."
- **Otherwise**:
  1. **Verify git repository**: `git rev-parse --git-dir`
  2. **Get current status**: `git log -1 --oneline`, `git branch --show-current`, `git remote -v`

### Expected Output

After completing the operation, provide a summary:

#### For New Clone

```text
✓ Successfully cloned repository!

Repository: <repo_name>
Location: $DEST_DIR/<repo_name>
Branch: <branch_name>
Latest commit: <commit_hash> - <commit_message>
Clone type: shallow
```

#### For Update (Pull)

```text
✓ Successfully updated repository!

Repository: <repo_name>
Location: $DEST_DIR/<repo_name>
Branch: <branch_name>
Commits pulled: <number>
Latest commit: <commit_hash> - <commit_message>
```

## Reference Documentation

- **[Examples](references/examples/)** -- Worked examples demonstrating common cloning scenarios

## Examples

Detailed examples are available in the `references/examples/` directory:

1. [basic-clone.md](references/examples/basic-clone.md) - Clone a public repository to a specific directory
2. [clone-branch.md](references/examples/clone-branch.md) - Clone specific branch with complete git history
3. [update-existing.md](references/examples/update-existing.md) - Update existing repository with fetch and pull

## Error Handling

Handle specific error scenarios gracefully by reporting the error and stopping execution:

### Network Errors

- **Symptoms**: "Could not resolve host", "Failed to connect"
- **Action**: Report error with suggestion to check internet connection and verify URL

### Authentication Errors

- **Symptoms**: "Authentication failed", "Permission denied"
- **Action**: Report error with suggestion to check SSH keys or credential helpers

### Disk Space Issues

- **Symptoms**: "No space left on device"
- **Action**: Report error with suggestion to free disk space

### Permission Errors

- **Symptoms**: "Permission denied" on directory creation/write
- **Action**: Report error with suggestion to check directory permissions

### Repository Not Found

- **Symptoms**: "Repository not found", "404"
- **Action**: Report error with suggestion to verify URL and check if repo is private

## Verification Checklist

Before completing, verify:

- [ ] Repository cloned successfully to `$DEST_DIR/<repo_name>`
- [ ] Correct branch checked out (if specified)
- [ ] Clone type matches request (shallow vs full history)
- [ ] Repository contains expected source files
- [ ] No authentication errors or partial clones
- [ ] For updates: working directory is clean, pull succeeded
- [ ] If detach was used: only `.git` was removed; `.gitignore` and `.gitattributes` (if present) are still in the tree

## Important Notes

- This skill runs **autonomously** -- it never prompts for user input
- Missing required arguments cause immediate failure with a descriptive error message
- Ambiguous situations (uncommitted changes, non-git directories) fail with clear errors rather than asking the user
- All git commands use non-interactive mode
- Authentication should be pre-configured (SSH keys or credential helpers)
- Default clone is shallow (`--depth 1`) for speed; use full clone examples when history is needed

Complete this task using the Shell tool to execute the necessary git commands with appropriate permissions (network access required).
