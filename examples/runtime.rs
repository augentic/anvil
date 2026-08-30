//! Scripted-model host for the mock-source journey: the engine loads the
//! built mock component by path through the deployment loader, e.g.
//! `specify ./target/wasm32-wasip2/release/examples/source.wasm`.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::collections::VecDeque;
        use std::future::Future;
        use std::sync::{Arc, Mutex};

        use omnia::MountAcquire;
        use omnia_filesystem::{Client as Filesystem, ConnectOptions as FilesystemOptions};
        use omnia_wasi_blobstore::WasiBlobstore;
        use omnia_wasi_keyvalue::WasiKeyValue;
        use omnia_wasi_model::{Answer, FutureResult, Request, ToolHost, WasiModel, WasiModelCtx};
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            guests: [
                {
                    id: "emery",
                    source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.cwasm")),
                },
            ],
            mounts: [
                { name: ".", path: "." },
            ],
            plugins: {
                interfaces: ["emery:adapter/source@0.1.0"],
                acquire: MountAcquire,
            },
            hosts: {
                WasiOtel: OtelDefault,
                WasiModel: ScriptedModel,
                // Same compiled-in store root as the shipped binary.
                WasiKeyValue: Filesystem(FilesystemOptions { root: ".omnia/storage".into() }),
                WasiBlobstore: Filesystem(FilesystemOptions { root: ".omnia/storage".into() }),
            }
        });

        const ANSWERS: [&str; 2] = [
            include_str!("../tests/specify/1-spec.md"),
            include_str!("../tests/specify/2-design.md"),
        ];

        /// FIFO host-side model script over the fixed answers.
        #[derive(Clone, Debug)]
        struct ScriptedModel(Arc<Mutex<VecDeque<&'static str>>>);

        impl omnia::Backend for ScriptedModel {
            type ConnectOptions = omnia::NoOptions;

            fn connect_with(
                _options: omnia::NoOptions,
            ) -> impl Future<Output = anyhow::Result<Self>> {
                std::future::ready(Ok(Self(Arc::new(Mutex::new(ANSWERS.into())))))
            }
        }

        impl WasiModelCtx for ScriptedModel {
            fn complete(
                &self, _request: Request, _tool_host: Arc<dyn ToolHost>,
            ) -> FutureResult<Answer> {
                let next = self.0.lock().expect("model script").pop_front();
                Box::pin(async move {
                    let answer = next.ok_or_else(|| anyhow::anyhow!("model script exhausted"))?;
                    Ok(Answer {
                        value: answer.into(),
                        usage: None,
                        transcript: None,
                    })
                })
            }
        }
    } else {
        fn main() {}
    }
}
