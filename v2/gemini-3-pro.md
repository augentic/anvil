# Multi-Repo OpenSpec Control Architecture

This proposal outlines a **Control Repo** architecture where a central repository manages specifications and orchestrates changes across multiple service repositories.

### Core Concept
*   **Control Repo (`platform-specs`)**: The single source of truth for specs, designs, and change management.
*   **Target Repos**: The actual service repositories (e.g., `frontend`, `backend`) where code lives.
*   **Dispatch Mechanism**: A custom `apply` script that parses the central `tasks.md` and dispatches instructions to agents in target repos.

### 1. Repository Structure

Create a new repository named `platform-specs`.

```text
platform-specs/
├── openspec/
│   ├── config.yaml          # Global config & rules
│   ├── schemas/             # Custom multi-repo schema
│   ├── specs/               # Centralized specs (Source of Truth)
│   │   ├── auth/            #   e.g. specs/auth/spec.md
│   │   └── billing/         #   e.g. specs/billing/spec.md
│   └── changes/             # Active proposals (OPSX artifacts)
├── repos.yaml               # Registry of repos
├── scripts/
│   ├── apply.js             # The multi-repo dispatcher
│   └── setup.sh             # Init script
└── package.json             # Dependencies for scripts
```

### 2. Central Registry (`repos.yaml`)

Define your repositories and their local paths (assuming a flat workspace structure).

```yaml
# repos.yaml
repos:
  frontend:
    path: ../frontend-app
    description: "Next.js web application"
  backend:
    path: ../backend-api
    description: "Go API service"
  shared-lib:
    path: ../shared-lib
    description: "Shared utilities"
```

### 3. Configuration & Schema (`openspec/`)

We need to enforce that tasks are grouped by repository so our script can parse them.

**`openspec/config.yaml`**
```yaml
schema: multi-repo
context: |
  This is a multi-repo platform.
  All changes must consider impact across services.
  
  Repositories:
  - frontend: Next.js web app
  - backend: Go API service
  - shared-lib: Shared utilities

rules:
  tasks:
    - Group tasks strictly by repository using "## Repo: <name>" headers
    - Use repository names defined in the context
    - Do not mix tasks from different repos in one section
```

**`openspec/schemas/multi-repo/schema.yaml`**
(Forked from `spec-driven` with modified `tasks` template/instruction)

```yaml
name: multi-repo
artifacts:
  - id: proposal
    generates: proposal.md
    requires: []
  - id: specs
    generates: specs/**/*.md
    requires: [proposal]
  - id: design
    generates: design.md
    requires: [proposal]
  - id: tasks
    generates: tasks.md
    requires: [specs, design]
    instruction: |
      Create a checklist of implementation tasks.
      CRITICAL: You MUST group tasks by repository using the format:
      
      ## Repo: frontend
      - [ ] Task 1
      
      ## Repo: backend
      - [ ] Task 2
```

### 4. The Workflow Implementation

#### Step 1: Propose (`/opsx:propose`)
Run this in the `platform-specs` repo.
*   **User**: `/opsx:propose "Add generic webhooks"`
*   **Agent**: Generates `proposal.md`, `specs/`, `design.md`.
*   **Agent**: Generates `tasks.md` grouped by repo (enforced by schema/config).

#### Step 2: Apply (`node scripts/apply.js`)
Instead of the standard `/opsx:apply` (which only works locally), use this script to dispatch work.

**`scripts/apply.js`**
```javascript
const fs = require('fs');
const path = require('path');
const yaml = require('js-yaml');
const { execSync } = require('child_process');

// 1. Load Registry
const repos = yaml.load(fs.readFileSync('repos.yaml', 'utf8')).repos;

// 2. Find Active Change (simplified)
// In a real script, prompt user to select from openspec/changes/
const changeDir = 'openspec/changes/add-webhooks'; 
const tasksFile = path.join(changeDir, 'tasks.md');
const contextFiles = ['proposal.md', 'design.md', 'specs'];

// 3. Parse Tasks
const content = fs.readFileSync(tasksFile, 'utf8');
const repoSections = content.split(/^## Repo: /m).slice(1);

repoSections.forEach(section => {
  const [repoName, ...lines] = section.split('\n');
  const taskList = lines.join('\n').trim();
  const repoConfig = repos[repoName.trim()];

  if (!repoConfig) {
    console.warn(`Unknown repo: ${repoName}`);
    return;
  }

  console.log(`🚀 Dispatching to ${repoName}...`);

  // 4. Prepare Context for Target Repo
  const prompt = `
    You are implementing a feature for ${repoName}.
    
    CONTEXT:
    ${fs.readFileSync(path.join(changeDir, 'proposal.md'), 'utf8')}
    
    DESIGN:
    ${fs.readFileSync(path.join(changeDir, 'design.md'), 'utf8')}
    
    YOUR TASKS:
    ${taskList}
    
    Implement these tasks in this repository.
  `;

  // 5. Execute Agent (Example using a CLI tool, or just print for user)
  // Ideally, use an MCP tool or CLI that accepts a prompt and path
  console.log(`\n--- Run this in ${repoConfig.path} ---\n`);
  console.log(prompt);
  console.log(`\n-------------------------------------\n`);
  
  // Optional: Automate if you have a CLI agent
  // execSync(`cursor --query "${prompt}" ${repoConfig.path}`);
});
```

#### Step 3: Archive (`/opsx:archive`)
Run this in `platform-specs` when all repos are updated.
*   **User**: `/opsx:archive`
*   **Agent**: Merges delta specs into `openspec/specs/` and moves the change folder to `openspec/changes/archive/`.

### Summary of Operations

| Action | Command | Location |
| :--- | :--- | :--- |
| **Plan** | `/opsx:propose "feature"` | `platform-specs` |
| **Implement** | `node scripts/apply.js` | `platform-specs` (dispatches to others) |
| **Verify** | (Manual/CI in target repos) | Target Repos |
| **Finalize** | `/opsx:archive` | `platform-specs` |
