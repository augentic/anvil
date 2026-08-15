//! Reviewed definition-home minter for the wasm example: writes the
//! degenerate greeting definition (`--from`/`--wave` input for
//! `emery plan author`) with the delivery target bound to the given
//! origin locator. (wasm32 builds compile an empty stub so
//! `--examples` passes.)

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(root), Some(locator)) = (args.next(), args.next()) else {
        eprintln!("usage: definition <root> <target-locator>");
        return std::process::ExitCode::FAILURE;
    };
    let spec = mock::definition::Spec {
        definition: "demo".into(),
        wave: "deliver".into(),
        outcome: "Deliver the reviewed greeting".into(),
        targets: vec![mock::definition::Target {
            id: "app".into(),
            locator,
            adapter: "target".into(),
        }],
        scopes: vec![mock::definition::Scope {
            source: "main".into(),
            adapter: "source".into(),
            location: String::new(),
            lead: "greeting".into(),
            value: Some("The greeting service.".into()),
        }],
        mappings: vec![mock::definition::Mapping {
            source: "main".into(),
            lead: "greeting".into(),
            target: "app".into(),
        }],
    };
    match mock::definition::mint(std::path::Path::new(&root), &spec) {
        Ok(minted) => {
            println!("definition {} wave {} handoff {}", root, minted.wave, minted.digest);
            std::process::ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("definition mint failed: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}
