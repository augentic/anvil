//! The shipped `emery` executable: the runtime `src/lib.rs` declares.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        fn main() {}
    } else {
        fn main() -> std::process::ExitCode {
            emery::main()
        }
    }
}
