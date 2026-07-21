//! The shipped `specify` executable: the RFC-70 deployment launcher
//! over the Omnia runtime.
//!
//! `main` asks the launcher to derive, hydrate, and verify the
//! invocation's component closure, then runs the typed deployment
//! through the nested Omnia host — argv and the engine guest's exit
//! code pass through byte-for-byte. Help/version displays and the
//! host-side `adapter add` seed complete inside the launcher without
//! a deployment. There is no `omnia.toml` and no `run --config`
//! surface: the deployment exists only in memory, per invocation.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::process::ExitCode;

        mod host {
            //! The Omnia command-mode runtime the launcher hands its
            //! deployment to. Specify-vocabulary-free: hosts and
            //! backends only.
            use omnia_cursor::Client as Cursor;
            use omnia_wasi_http::{HttpDefault, WasiHttp};
            use omnia_wasi_model::WasiModel;
            use omnia_wasi_otel::{OtelDefault, WasiOtel};

            omnia::runtime!({
                mode: command,
                hosts: {
                    WasiHttp: HttpDefault,
                    WasiOtel: OtelDefault,
                    WasiModel: Cursor,
                }
            });
        }

        /// The engine identity this binary supplies to the launcher:
        /// its own version (the engine guest is versioned by the
        /// binary). The launcher hydrates `specify:engine@<version>`
        /// from the registry on a store miss.
        const ENGINE: launcher::Engine =
            launcher::Engine { version: env!("CARGO_PKG_VERSION") };

        fn main() -> ExitCode {
            let argv: Vec<String> = std::env::args().skip(1).collect();
            let invoked_dir = match std::env::current_dir() {
                Ok(dir) => dir,
                Err(err) => {
                    eprintln!("error: cannot resolve the working directory: {err}");
                    return ExitCode::FAILURE;
                }
            };
            let deployment = match launcher::prepare(&invoked_dir, &argv, ENGINE) {
                launcher::Outcome::Run(deployment) => deployment,
                launcher::Outcome::Done { stdout, stderr, code } => {
                    use std::io::Write as _;
                    drop(std::io::stdout().write_all(&stdout));
                    drop(std::io::stderr().write_all(&stderr));
                    return ExitCode::from(code);
                }
            };
            let builder = omnia::DeploymentBuilder::new().manifest(manifest(deployment)).args(argv);
            match host::run(builder) {
                Ok(status) => ExitCode::from(status),
                Err(err) => {
                    eprintln!("error: {err:#}");
                    ExitCode::FAILURE
                }
            }
        }

        /// Map the launcher's typed deployment onto the Omnia manifest.
        fn manifest(deployment: launcher::Deployment) -> omnia::Manifest {
            let mut manifest = omnia::Manifest::new().guest(guest(deployment.engine));
            for adapter in deployment.adapters {
                manifest = manifest.guest(guest(adapter));
            }
            manifest.mounts(deployment.mounts.into_iter().map(|mount| omnia::Mount {
                name: mount.name,
                path: mount.path,
                writable: mount.writable,
            }))
        }

        fn guest(guest: launcher::Guest) -> omnia::GuestEntry {
            let mut entry = omnia::GuestEntry::new(guest.id, guest.component);
            for interface in guest.links {
                entry = entry.link(interface);
            }
            entry
        }
    } else {
        fn main() {}
    }
}
