//! Integration tests for manifest-declared native build hooks.

use std::fs;

use specify_error::Error;
use specify_workflow::adapter::NativeBuildHookDeclaration;
use specify_workflow::slice::run_native_build_hook;
use tempfile::tempdir;

#[test]
fn runs_successful_hook_script() {
    let adapter = tempdir().expect("tempdir");
    let scripts = adapter.path().join("scripts");
    fs::create_dir_all(&scripts).expect("mkdir scripts");
    let script = scripts.join("ok.sh");
    fs::write(&script, "#!/bin/sh\nexit 0\n").expect("write script");

    let project = tempdir().expect("project");
    let slice = project.path().join(".specify/slices/demo");
    fs::create_dir_all(&slice).expect("mkdir slice");

    run_native_build_hook(
        adapter.path(),
        &NativeBuildHookDeclaration {
            script: "scripts/ok.sh".into(),
        },
        project.path(),
        &slice,
        "target-build-host-prereq-missing",
        "host prereq hook passes",
    )
    .expect("hook ok");
}

#[test]
fn rejects_parent_dir_escape() {
    let adapter = tempdir().expect("tempdir");
    let err = run_native_build_hook(
        adapter.path(),
        &NativeBuildHookDeclaration {
            script: "../escape.sh".into(),
        },
        adapter.path(),
        adapter.path(),
        "target-build-host-prereq-missing",
        "host prereq hook passes",
    )
    .expect_err("must reject escape");
    let Error::Validation { code, .. } = err else {
        panic!("expected validation error");
    };
    assert_eq!(code, "adapter-manifest-invalid");
}

#[test]
fn runs_script_with_leading_dash_in_name() {
    let adapter = tempdir().expect("tempdir");
    let script = adapter.path().join("-hook.sh");
    fs::write(&script, "#!/bin/sh\nexit 0\n").expect("write script");

    let project = tempdir().expect("project");
    let slice = project.path().join(".specify/slices/demo");
    fs::create_dir_all(&slice).expect("mkdir slice");

    run_native_build_hook(
        adapter.path(),
        &NativeBuildHookDeclaration {
            script: "-hook.sh".into(),
        },
        project.path(),
        &slice,
        "target-build-host-prereq-missing",
        "host prereq hook passes",
    )
    .expect("dash-prefixed script name must not be parsed as sh option");
}

#[test]
fn rejects_bash_only_syntax_under_sh() {
    let adapter = tempdir().expect("tempdir");
    let scripts = adapter.path().join("scripts");
    fs::create_dir_all(&scripts).expect("mkdir scripts");
    let script = scripts.join("bash-only.sh");
    fs::write(&script, "#!/usr/bin/env bash\nwhile read -r x; do :; done < <(echo x)\n")
        .expect("write script");

    let project = tempdir().expect("project");
    let slice = project.path().join(".specify/slices/demo");
    fs::create_dir_all(&slice).expect("mkdir slice");

    let err = run_native_build_hook(
        adapter.path(),
        &NativeBuildHookDeclaration {
            script: "scripts/bash-only.sh".into(),
        },
        project.path(),
        &slice,
        "target-build-host-prereq-missing",
        "host prereq hook passes",
    )
    .expect_err("bash-only syntax must fail under sh");
    let Error::Validation { code, .. } = err else {
        panic!("expected validation error");
    };
    assert_eq!(code, "target-build-host-prereq-missing");
}
