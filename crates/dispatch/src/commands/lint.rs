//! `specify lint framework` — framework CI tooling (`make lint`),
//! hidden from operator help. Runs in the workflow guest like every
//! other verb, walking the framework repo through the `"."` mount.

pub mod cli;
pub mod framework;
