//! Compiles the mock adapter example (`examples/adapter`) to a
//! `wasm32-wasip2` component and generates `gen.rs` with its artifact path,
//! through omnia-test's fixture pipeline (a no-op under a `wasm32` outer
//! target). The tracked paths are inputs the nested build's dep-info misses.

fn main() {
    omnia_test::build::Components::in_workspace("../..")
        .examples(["adapter"])
        .track(["examples/adapter", "crates/adapter", "wit", "Cargo.lock"])
        .build()
        .write_gen("gen.rs");
}
