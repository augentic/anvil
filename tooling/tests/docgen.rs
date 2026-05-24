use tooling::context::Context;
use tooling::docgen::{render_generated, splice_generated, BEGIN_MARKER, END_MARKER};

#[test]
fn render_generated_matches_deno_splice_shape() {
    let ctx = Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root");
    let specify_cli_dir = ctx.specify_cli_dir();
    if !specify_cli_dir.join("tests/fixtures/plan").is_dir() {
        eprintln!(
            "skip render_generated_matches_deno_splice_shape: {} not found",
            specify_cli_dir.display()
        );
        return;
    }

    let generated = render_generated(&specify_cli_dir).expect("render generated block");
    assert!(!generated.is_empty());
    assert!(generated.contains("### `specify plan"));
    assert!(generated.contains("Source fixture: `tests/fixtures/"));
}

#[test]
fn envelopes_check_passes_on_current_doc_after_regenerate() {
    let ctx = Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root");
    let specify_cli_dir = ctx.specify_cli_dir();
    if !specify_cli_dir.join("tests/fixtures/plan").is_dir() {
        eprintln!(
            "skip envelopes_check_passes_on_current_doc_after_regenerate: {} not found",
            specify_cli_dir.display()
        );
        return;
    }

    let doc_path = ctx.framework_root().join("docs/reference/cli-output-shapes.md");
    let current = std::fs::read_to_string(&doc_path).expect("read cli-output-shapes.md");
    let generated = render_generated(&specify_cli_dir).expect("render generated block");
    let next = splice_generated(&current, &generated).expect("splice generated block");

    if next != current {
        eprintln!(
            "skip envelopes_check_passes_on_current_doc_after_regenerate: doc drift; run docgen envelopes first"
        );
        return;
    }

    let exit = tooling::docgen::run_envelopes(ctx.framework_root(), &specify_cli_dir, true)
        .expect("run envelopes --check");
    assert_eq!(exit, tooling::Exit::Success);
}

#[test]
fn envelopes_check_fails_on_drift() {
    let ctx = Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root");
    let specify_cli_dir = ctx.specify_cli_dir();
    if !specify_cli_dir.join("tests/fixtures/plan").is_dir() {
        eprintln!(
            "skip envelopes_check_fails_on_drift: {} not found",
            specify_cli_dir.display()
        );
        return;
    }

    let doc_path = ctx.framework_root().join("docs/reference/cli-output-shapes.md");
    let current = std::fs::read_to_string(&doc_path).expect("read cli-output-shapes.md");
    let generated = render_generated(&specify_cli_dir).expect("render generated block");
    let next = splice_generated(&current, &generated).expect("splice generated block");
    if next == current {
        eprintln!("skip envelopes_check_fails_on_drift: doc already matches fixtures");
        return;
    }

    let exit = tooling::docgen::run_envelopes(ctx.framework_root(), &specify_cli_dir, true)
        .expect("run envelopes --check");
    assert_eq!(exit, tooling::Exit::ValidationFailed);
}

#[test]
fn doc_contains_generation_markers() {
    let ctx = Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root");
    let doc_path = ctx.framework_root().join("docs/reference/cli-output-shapes.md");
    let current = std::fs::read_to_string(&doc_path).expect("read cli-output-shapes.md");
    assert!(current.contains(BEGIN_MARKER));
    assert!(current.contains(END_MARKER));
}
