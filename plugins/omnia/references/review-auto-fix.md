# Auto-Fix Protocol

> **When to read this**: Read this only when the operator passed `--fix`. It defines which findings are eligible for automatic repair, the regression guard (`cargo check`), expected success rates per category, and the recovery procedure if auto-fix breaks the build.

## Step 6: Auto-Fix (if --fix flag provided)

If `$AUTO_FIX == true`:

The **lead** applies all auto-fixes directly (specialists and antagonist have completed their analysis at this point). The finding prefix (SEC-, COR-, QUA-, UNI-) tracks which reviewer or pass identified the issue for accountability in the report.

**FOR EACH** confirmed or upgraded auto-fixable issue (not disputed):

1. **Verify** fix is safe (no side effects, antagonist did not flag regression risk)
2. **Apply** fix using Edit tool
3. **Mark** issue as "Fixed" in report, noting the originating reviewer prefix
4. **Add** to auto-fix log

**RE-CHECK**: Run `cargo check` to verify fixes compile

```bash
cd $CRATE_PATH && cargo check 2>&1
```

If errors introduced:

- **REVERT** all auto-fixes
- **WARN** in report: "Auto-fix caused compilation errors; manual review required"

## Auto-Fix Success Rate (per category)

- **Error handling (unwrap→?)**: 90% success
- **WASM violations (std::env→Config)**: 80% success
- **Missing validation**: 60% success (some require business logic understanding)
- **Performance issues**: 10% success (most need architectural changes)
- **Code quality**: 5% success (semantic understanding required)

**Overall auto-fix rate**: ~40-50% of issues

## Common Issues and Resolutions

| Issue                              | Cause                                       | Resolution                                                               |
| ---------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------ |
| Crate path not found               | Incorrect `$CRATE_PATH` argument            | Verify the path exists and contains `src/` with `.rs` files              |
| No `.rs` files in `src/`           | Crate not yet generated or wrong directory  | Run `crate-writer` first, then re-run code review                        |
| `cargo check` fails after auto-fix | Auto-fix introduced a compilation error     | Revert all auto-fixes and document issues for manual resolution          |
| Review report empty                | All files excluded or no issues found       | Verify `src/` directory is not empty; check file permissions             |
| Auto-fix modifies test files       | Test code scanned alongside production code | Review should focus on `src/` only; exclude `tests/` from auto-fix scope |

## Recovery Process

1. If auto-fix caused compilation errors: revert changes with `git checkout -- src/`
2. Re-run review without `--fix` to get a report without auto-fixes
3. Apply fixes manually based on the report recommendations
4. Run `cargo check` and `cargo test` after each manual fix
5. Re-run review to verify issues are resolved

## Auto-Fix Verification Checklist

Before declaring an auto-fix pass complete:

- [ ] Only confirmed or upgraded auto-fixable issues fixed (not disputed)
- [ ] Antagonist regression flags respected (no fix applied if flagged)
- [ ] Lead applied all fixes (not delegated to specialists)
- [ ] All fixes verified with `cargo check`
- [ ] Modified files listed with originating prefix (SEC-, COR-, QUA-, UNI-)
- [ ] Revert performed if errors introduced
