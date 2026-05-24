pub mod check;
pub mod context;
pub mod docgen;
pub mod error;
pub mod exit;
pub mod finding;
pub mod helpers;
pub mod schema;
pub mod test_support;

pub use context::Context;
pub use error::ToolingError;
pub use exit::Exit;
pub use finding::{Check, Finding, Location};
pub use helpers::{
    skill_body_lines, skill_frontmatter, strip_html_comments, under_symlink, walk_matching_files,
    walk_skill_files,
};
pub use schema::{validate_frontmatter, validate_value, SchemaError, SchemaId, ValidationError};
