//! Integration coverage for the AGENTS.md write planner
//! (`specify_workflow::agents::fences::plan_agents_write`). Re-homed and
//! collapsed from the former `agents/fences/render.rs` unit module: one test
//! sweeps the four write dispositions plus non-UTF-8 byte preservation, the
//! other the two fence-policy errors. Every former input is represented.

use specify_workflow::agents::fences::{FenceError, WriteDisposition, plan_agents_write};

const GENERATED: &[u8] = b"# demo - Agent Instructions\n\n<!-- specify:context begin\nfingerprint: sha256:new\ngenerated-by: specify 0.2.0\n-->\n\n## Runtime\n- new\n\n<!-- specify:context end -->\n";

fn fenced_existing() -> Vec<u8> {
    b"# hand title\n\n<!-- specify:context begin\nfingerprint: sha256:old\n-->\n\nold body\n\n<!-- specify:context end -->\n\noperator notes\n".to_vec()
}

#[test]
fn plan_write_dispositions() {
    // Absent AGENTS.md → the full generated document is created.
    let planned = plan_agents_write(None, GENERATED, false).expect("plan ok");
    assert_eq!(planned.bytes, GENERATED);
    assert_eq!(planned.disposition, WriteDisposition::Create);

    // Unfenced existing + `--force` → full rewrite.
    let planned =
        plan_agents_write(Some(b"# hand-authored\n"), GENERATED, true).expect("force rewrite ok");
    assert_eq!(planned.bytes, GENERATED);
    assert_eq!(planned.disposition, WriteDisposition::ForceRewriteUnfenced);

    // Fenced existing → only the generated block is spliced, prefix/suffix kept.
    let existing = fenced_existing();
    let planned = plan_agents_write(Some(&existing), GENERATED, false).expect("plan ok");
    let expected = b"# hand title\n\n<!-- specify:context begin\nfingerprint: sha256:new\ngenerated-by: specify 0.2.0\n-->\n\n## Runtime\n- new\n\n<!-- specify:context end -->\n\noperator notes\n";
    assert_eq!(planned.bytes, expected);
    assert_eq!(planned.disposition, WriteDisposition::ReplaceFencedBlock);

    // Identical bytes → Unchanged.
    let planned = plan_agents_write(Some(GENERATED), GENERATED, false).expect("plan ok");
    assert_eq!(planned.bytes, GENERATED);
    assert_eq!(planned.disposition, WriteDisposition::Unchanged);

    // Non-UTF-8 operator bytes in the prefix/suffix are preserved verbatim.
    let existing = [
        b"prefix ".as_slice(),
        &[0xff, b'\n'],
        b"<!-- specify:context begin\nfingerprint: sha256:old\n-->\nold\n<!-- specify:context end -->",
        &[b'\n', 0xfe],
    ]
    .concat();
    let planned = plan_agents_write(Some(&existing), GENERATED, false).expect("plan ok");
    assert!(planned.bytes.starts_with(b"prefix \xff\n<!-- specify:context begin"));
    assert!(planned.bytes.ends_with(b"\n\xfe"));
}

#[test]
fn plan_write_errors() {
    // Unfenced existing without `--force` is refused.
    let err = plan_agents_write(Some(b"# hand-authored\n"), GENERATED, false)
        .expect_err("unfenced must refuse");
    assert_eq!(err, FenceError::ExistingUnfencedAgentsMd);

    // A generated document missing its own fences is rejected.
    let err = plan_agents_write(None, b"# generated but unfenced\n", false)
        .expect_err("generated document without fences must fail");
    assert_eq!(err, FenceError::GeneratedDocumentMissingFences);
}
