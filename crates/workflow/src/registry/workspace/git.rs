//! Git inspection for workspace slots.

use std::path::Path;

use crate::cmd;

pub(super) fn git_output_ok(tree: &Path, args: &[&str]) -> Option<String> {
    let output = cmd::git(&cmd::real_cmd, Some(tree), args).ok()?;
    if !output.status.success() {
        return None;
    }
    let output = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!output.is_empty()).then_some(output)
}
