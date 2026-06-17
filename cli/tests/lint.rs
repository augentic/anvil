//! End-to-end binary tests for the `specify lint` surface
//! (`lint framework`, `lint framework --format json`, and `lint
//! project`).

#[path = "lint/support.rs"]
mod support;

#[path = "lint/framework.rs"]
mod framework;

#[path = "lint/framework_adapters.rs"]
mod framework_adapters;

#[path = "lint/framework_json.rs"]
mod framework_json;

#[path = "lint/project.rs"]
mod project;
