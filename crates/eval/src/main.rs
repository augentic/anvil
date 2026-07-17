//! Native CLI and live eval over the linked adapters.

fn main() -> std::process::ExitCode {
    harness::entry::main::<Adapters>(None)
}

harness::adapters! {
    Adapters {
        source fixture::Docs,
        source fixture::Code,
        target fixture::Adapter,
    }
}
