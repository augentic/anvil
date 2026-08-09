# The guest's `wasi:blobstore` client WIT

`world.wit` declares the emery-named bindgen world (`emery:blob-client`
/ `snapshots`); `deps/` holds byte-for-byte copies of the WIT Omnia's
`wasi-blobstore` host crate vendors (`crates/wasi-blobstore/wit/` in
`augentic/omnia`), so the engine guest's imports resolve against the
exact interface identities the shipped runtime links. The world is
emery-named because Omnia's own blobstore guest bindings are linked
into the same binary: an identically named world would make the linker
concatenate the two encoded-world custom sections, corrupting both.

The guest bindgen over this world forces sync lowering
(`async: false`): the workspace kernel is synchronous, and every
blobstore leg is quick local object I/O.

Re-vendor `deps/` when the Omnia pin's copy changes; the upstream
package (`wasi:blobstore@0.2.0-draft`) is frozen.
