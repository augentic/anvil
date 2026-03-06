use std::path::Path;
use std::process::Command;

#[test]
fn status_command_works_with_example_change() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_root = repo_root.join("examples/openspec");
    let temp_root = std::env::temp_dir().join(format!("opsx-e2e-{}", std::process::id()));
    if temp_root.exists() {
        std::fs::remove_dir_all(&temp_root).expect("remove old temp dir");
    }
    std::fs::create_dir_all(&temp_root).expect("create temp dir");

    std::fs::copy(fixture_root.join("registry.toml"), temp_root.join("registry.toml"))
        .expect("copy registry.toml");
    std::fs::create_dir_all(temp_root.join("openspec")).expect("create openspec dir");
    std::fs::copy(fixture_root.join("config.yaml"), temp_root.join("openspec/config.yaml"))
        .expect("copy openspec config");
    copy_dir_recursive(
        &fixture_root.join("changes/r9k-http"),
        &temp_root.join("openspec/changes/r9k-http"),
    )
    .expect("copy change fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_opsx"))
        .current_dir(&temp_root)
        .args(["status", "r9k-http"])
        .output()
        .expect("run opsx status");

    assert!(
        output.status.success(),
        "status command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("r9k-connector"));
    assert!(stdout.contains("r9k-adapter"));

    std::fs::remove_dir_all(&temp_root).expect("cleanup temp dir");
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}
