use std::path::Path;

use tooling::Context;

#[test]
fn framework_root_resolves_from_manifest_dir() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let ctx = Context::from_manifest_dir(manifest_dir).expect("framework root resolves");
    let expected = Path::new(manifest_dir)
        .parent()
        .expect("tooling crate has a parent directory");
    assert_eq!(ctx.framework_root(), expected);
    assert!(ctx.plugins_dir().join("spec").is_dir());
    assert!(ctx.specify_cli_schemas_dir().ends_with("schemas"));
}

#[test]
fn check_runs_with_no_findings() {
    let ctx =
        Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root resolves");
    let findings = tooling::check::run(&ctx);
    assert!(findings.is_empty());
}
