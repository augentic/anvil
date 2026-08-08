# `emery:workspaces` WIT

This crate owns [`workspaces.wit`](workspaces.wit) — the authoritative
definition of the `emery:workspaces@0.1.0` package (the `workspaces`
interface and the `workspace-host` bindgen world).

The engine guest's `workflow` world imports the interface through the
repo-root `wit/deps/workspaces` symlink; adapters never see this
package. Publishing is not required for the shipped binary (host and
guest compile against the in-tree copy).
