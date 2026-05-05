# Greenfield bootstrap fixture

Pins the `specify workspace sync` greenfield fallback sequence:

1. Clone attempt fails (repo does not exist)
2. `mkdir -p .specify/workspace/<name>/`
3. `git init`
4. `git remote add origin <url>`
5. `specify init <capability>`
6. `git add . && git commit -m "Initial Specify scaffold"`

The result is a valid Specify project with `.specify/project.yaml`.
