use std::path::PathBuf;

use super::*;

#[test]
fn tasks_parser_collects_paths() {
    let text = "## Tasks\n\n- intro\n\n## Touches\n\n- crates/billing/src/lib.rs\n* docs/billing.md\n\n## Notes\n\n- unrelated\n";
    let paths = parse_slice_tasks_paths(text);
    assert_eq!(
        paths,
        vec![PathBuf::from("crates/billing/src/lib.rs"), PathBuf::from("docs/billing.md"),]
    );
}

#[test]
fn tasks_parser_touches_and_produces() {
    let text = "## Produces\n\n- a.md\n\n## Touches\n\n- b.md\n";
    let paths = parse_slice_tasks_paths(text);
    assert_eq!(paths, vec![PathBuf::from("a.md"), PathBuf::from("b.md")]);
}
