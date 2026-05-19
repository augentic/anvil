# specify CLI commands (post RFC-25 + RFC-26)

Specify 3.0 target surface. Global on all rows: `--format text|json` (`SPECIFY_FORMAT`).

| Command | Positionals | Flags |
|---------|-------------|-------|
| `specify init` | `<target>` | `--name`, `--domain` |
| `specify init` | | `--hub`, `--name`, `--domain` |
| `specify status` | | |
| `specify context generate` | | `--check`, `--force` |
| `specify context check` | | |
| `specify source resolve` | `<name>` | `--project-dir` |
| `specify source list` | | |
| `specify source validate` | `<name>` | |
| `specify target resolve` | `<value>` | `--project-dir` |
| `specify target list` | | |
| `specify target validate` | `<name>` | |
| `specify target pipeline build` | | `--slice` |
| `specify target pipeline merge` | | `--slice` |
| `specify upgrade` | | |
| `specify plan create` | `<name>` | `--source` |
| `specify plan show` | | |
| `specify plan finalize` | | `--clean`, `--dry-run` |
| `specify plan validate` | | |
| `specify plan next` | | |
| `specify plan status` | | |
| `specify plan add` | `<name>` | `--depends-on`, `--sources`, `--description`, `--project`, `--target`, `--context` |
| `specify plan amend` | `<name>` | `--depends-on`, `--sources`, `--description`, `--project`, `--target`, `--context`, `--add-source`, `--remove-source` |
| `specify plan transition` | `<name>`, `<target>` | `--reason` |
| `specify plan archive` | | `--force` |
| `specify plan lock acquire` | | `--pid` |
| `specify plan lock release` | | `--pid` |
| `specify plan lock status` | | |
| `specify slice create` | `<name>` | `--target`, `--if-exists` |
| `specify slice status` | `<name>` | |
| `specify slice validate` | `<name>` | |
| `specify slice transition` | `<name>`, `<target>` | `--reason` |
| `specify slice synthesize pipeline` | | `--change` |
| `specify slice merge preview` | `<name>` | |
| `specify slice merge conflict-check` | `<name>` | |
| `specify slice merge run` | `<name>` | |
| `specify slice task progress` | `<name>` | |
| `specify slice task mark` | `<name>`, `<task-id>` | |
| `specify slice outcome set success` | `<name>`, `<phase>` | `--summary`, `--context` |
| `specify slice outcome set failure` | `<name>`, `<phase>` | `--summary`, `--context` |
| `specify slice outcome set deferred` | `<name>`, `<phase>` | `--summary`, `--context` |
| `specify slice outcome set registry-amendment-required` | `<name>`, `<phase>` | `--summary`, `--context`, `--proposal` |
| `specify slice outcome show` | `<name>` | |
| `specify slice journal append` | `<name>`, `<phase>`, `<kind>` | `--summary`, `--context` |
| `specify slice journal show` | `<name>` | |
| `specify slice touched-specs` | `<name>` | `--scan`, `--set` |
| `specify slice overlap` | `<name>` | |
| `specify slice drop` | `<name>` | `--reason` |
| `specify registry show` | | |
| `specify registry validate` | | |
| `specify registry add` | `<name>` | `--url`, `--target`, `--description` |
| `specify registry remove` | `<name>` | |
| `specify workspace sync` | `[<project>…]` | |
| `specify workspace status` | `[<project>…]` | |
| `specify workspace push` | `[<project>…]` | `--dry-run` |
| `specify workspace prepare-branch` | `<project>` | `--change`, `--source`, `--output` |
| `specify tool run` | `<name>`, `[args…]` | arguments after `--` |
| `specify tool list` | | |
| `specify tool fetch` | `[<name>]` | |
| `specify tool show` | `<name>` | |
| `specify tool gc` | | |
| `specify compatibility check` | | `--change`, `--report-only` |
| `specify codex export` | | |
| `specify completions` | `<shell>` | |

`<target>` for `specify plan transition`: plan lifecycle `reviewed`; per-entry `pending`, `in-progress`, `done`, `blocked`, `failed`, `skipped` (`--reason` only for `failed`, `blocked`, `skipped`). `<target>` for `specify slice transition`: `defining`, `defined_provisional`, `defined`, `built`, `dropped`. `<phase>`: `define`, `build`, `merge`. `<kind>`: `question`, `failure`, `recovery`. `<shell>`: `bash`, `elvish`, `fish`, `powershell`, `zsh`. Repeatable flags: `plan create --source`, `plan add`/`amend` `--depends-on`/`--sources`/`--context`, `workspace prepare-branch` `--source`/`--output`. Retired: `specify adapter *`, `specify change *`, `specify change survey`, `specify plan doctor`.
