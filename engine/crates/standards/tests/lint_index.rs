//! Integration tests for the `lint::index::build` workspace indexer and
//! the `WorkspaceModel` it produces (project + framework profiles,
//! scenario discovery, and DTO round-trip).

mod common;

#[path = "lint_index/framework_indexer.rs"]
mod framework_indexer;
#[path = "lint_index/index_scenario.rs"]
mod index_scenario;
#[path = "lint_index/indexer_project.rs"]
mod indexer_project;
#[path = "lint_index/model_round_trip.rs"]
mod model_round_trip;
