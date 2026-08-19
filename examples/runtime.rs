//! The journey host (ADR-0009 §5): the shipped runtime shape with one
//! substitution — `WasiModel` answers from a script directory instead
//! of the Cursor backend. Never shipped.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::sync::Arc;

        use anyhow::Context as _;
        use omnia_testkit::model::Scripted;
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_model::{Answer, FutureResult, Request, ToolHost, WasiModel, WasiModelCtx};
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        /// Environment variable naming the script directory: every file in
        /// it, sorted by name, is one model answer in dispatch order.
        const SCRIPT_ENV: &str = "EMERY_JOURNEY_SCRIPT";

        /// The scripted `wasi:model` backend behind the unchanged seam.
        #[derive(Clone, Debug)]
        struct ScriptedModel(Scripted);

        impl omnia::Backend for ScriptedModel {
            type ConnectOptions = omnia::NoOptions;

            async fn connect_with(_options: omnia::NoOptions) -> anyhow::Result<Self> {
                let dir = std::env::var(SCRIPT_ENV)
                    .with_context(|| format!("{SCRIPT_ENV} must name the model script directory"))?;
                let mut files: Vec<_> = std::fs::read_dir(&dir)
                    .with_context(|| format!("reading the model script directory `{dir}`"))?
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .map(|entry| entry.path())
                    .filter(|path| path.is_file())
                    .collect();
                files.sort();
                let answers: Vec<String> = files
                    .iter()
                    .map(std::fs::read_to_string)
                    .collect::<Result<_, _>>()
                    .context("reading a script answer")?;
                anyhow::ensure!(
                    !answers.is_empty(),
                    "the script directory `{dir}` carries no answers"
                );
                Ok(Self(Scripted::answers(answers)))
            }
        }

        impl WasiModelCtx for ScriptedModel {
            fn complete(
                &self, request: Request, tool_host: Arc<dyn ToolHost>,
            ) -> FutureResult<Answer> {
                self.0.complete(request, tool_host)
            }
        }

        omnia::runtime!({
            mode: command,
            program: "emery",
            guests: [{
                id: "emery",
                // The root `build.rs` already embeds this for the shipped
                // binary; the example shares the same `OUT_DIR`.
                source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.bin")),
            }],
            mounts: [
                { name: ".", path: launcher::project_root(), writable: true },
                { name: launcher::CACHE_MOUNT, path: launcher::cache_dir(), writable: true },
            ],
            link: ["emery:adapter/source@0.1.0"],
            resolver: launcher::resolver(),
            http_paths: launcher::mcp_route,
            http_listener: launcher::http_listener(),
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiModel: ScriptedModel,
            }
        });
    } else {
        fn main() {}
    }
}
