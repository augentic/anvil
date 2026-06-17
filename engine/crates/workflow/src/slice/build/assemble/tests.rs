use super::*;

fn slice_tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let tree = dir.path();
    fs::create_dir_all(tree.join("specs/identity")).expect("mkdir specs");
    fs::write(tree.join("specs/identity/spec.md"), "# spec").expect("write spec");
    fs::write(tree.join("proposal.md"), "# proposal").expect("write proposal");
    fs::write(tree.join("design.md"), "# design").expect("write design");
    fs::write(tree.join("tasks.md"), "# tasks").expect("write tasks");
    dir
}

#[test]
fn assembles_request_with_present_input() {
    let dir = slice_tree();
    let tree = dir.path();
    fs::write(tree.join("tokens.yaml"), "tokens: {}").expect("write tokens");
    let inputs = vec![BuildInputDeclaration {
        path: "tokens.yaml".to_string(),
        required: true,
    }];

    let req = build_request("identity-service", &inputs, tree, Path::new("/work"))
        .expect("request assembles");

    assert_eq!(req.version, BUILD_VERSION);
    assert_eq!(req.slice, "identity-service");
    assert_eq!(req.project_dir, Path::new("/work"));
    assert_eq!(req.inputs.root, tree);
    assert_eq!(req.inputs.artifacts.proposal, "proposal.md");
    assert_eq!(req.inputs.artifacts.design, "design.md");
    assert_eq!(req.inputs.artifacts.tasks, "tasks.md");
    assert_eq!(req.inputs.artifacts.specs, vec!["specs/identity/spec.md".to_string()]);
    assert_eq!(req.inputs.artifacts.additional, vec!["tokens.yaml".to_string()]);

    // The assembled request is schema-valid.
    let json = serde_json::to_string(&req).expect("serialise request");
    crate::schema::validate_build_request_json(&json).expect("assembled request validates");
}

#[test]
fn missing_required_input_aborts() {
    let dir = slice_tree();
    let inputs = vec![BuildInputDeclaration {
        path: "tokens.yaml".to_string(),
        required: true,
    }];

    match build_request("identity-service", &inputs, dir.path(), Path::new("/work")) {
        Err(Error::Validation { code, .. }) => assert_eq!(code, "target-build-input-missing"),
        other => panic!("expected target-build-input-missing, got {other:?}"),
    }
}

#[test]
fn missing_optional_input_is_skipped() {
    let dir = slice_tree();
    let inputs = vec![BuildInputDeclaration {
        path: "assets.yaml".to_string(),
        required: false,
    }];

    let req = build_request("identity-service", &inputs, dir.path(), Path::new("/work"))
        .expect("request assembles");
    assert!(req.inputs.artifacts.additional.is_empty());
}
