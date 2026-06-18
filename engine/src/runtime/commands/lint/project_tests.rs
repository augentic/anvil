use std::path::PathBuf;

use super::*;

// `parse_slice_tasks_paths` collects `-` and `*` bullets under the path
// sections (Touches, Produces) and ignores every other section.
#[test]
fn tasks_parser_collects_section_paths() {
    assert_eq!(
        parse_slice_tasks_paths(
            "## Tasks\n\n- intro\n\n## Touches\n\n- crates/billing/src/lib.rs\n* docs/billing.md\n\n## Notes\n\n- unrelated\n"
        ),
        vec![PathBuf::from("crates/billing/src/lib.rs"), PathBuf::from("docs/billing.md")]
    );
    assert_eq!(
        parse_slice_tasks_paths("## Produces\n\n- a.md\n\n## Touches\n\n- b.md\n"),
        vec![PathBuf::from("a.md"), PathBuf::from("b.md")]
    );
}
