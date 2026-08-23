//! Static host for the mock source component with fixed synthesis answers.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::future::Future;
        use std::sync::Arc;

        use omnia_filesystem::Client as Filesystem;
        use omnia_testkit::model::Scripted;
        use omnia_wasi_blobstore::WasiBlobstore;
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_keyvalue::WasiKeyValue;
        use omnia_wasi_model::{Answer, FutureResult, Request, ToolHost, WasiModel, WasiModelCtx};
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            guests: [
                {
                    id: "emery",
                    source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.cwasm")),
                    routes: {http: ["/mcp/emery/spec"]},
                },
                {
                    id: "source:source",
                    source: env!("EMERY_SOURCE_WASM"),
                    routes: {http: ["/mcp/source/source"]},
                },
            ],
            mounts: [
                { name: ".", path: "." },
            ],
            dispatch: ["emery:adapter/source@0.1.0"],
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiModel: ScriptedModel,
                WasiKeyValue: Filesystem,
                WasiBlobstore: Filesystem,
            }
        });

        const ANSWERS: [&str; 2] = [
            include_str!("../tests/specify/1-spec.md"),
            include_str!("../tests/specify/2-design.md"),
        ];

        #[derive(Clone, Debug)]
        struct ScriptedModel(Scripted);

        impl omnia::Backend for ScriptedModel {
            type ConnectOptions = omnia::NoOptions;

            fn connect_with(
                _options: omnia::NoOptions,
            ) -> impl Future<Output = anyhow::Result<Self>> {
                std::future::ready(Ok(Self(Scripted::answers(ANSWERS))))
            }
        }

        impl WasiModelCtx for ScriptedModel {
            fn complete(
                &self, request: Request, tool_host: Arc<dyn ToolHost>,
            ) -> FutureResult<Answer> {
                self.0.complete(request, tool_host)
            }
        }
    } else {
        fn main() {}
    }
}
