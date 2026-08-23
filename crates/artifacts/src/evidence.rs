//! Source-adapter Evidence types.

pub mod authority;
pub mod claim;

pub use authority::{AuthorityClass, ClaimKind};
pub use claim::{Backing, Claim, ExampleClaim, validate_claims};

/// One kebab-case slug segment (`^[a-z0-9]+(-[a-z0-9]+)*$`).
///
/// This copy remains separate because the leaf adapter SDK cannot
/// depend on `artifacts`.
#[must_use]
pub fn is_kebab(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|segment| {
            !segment.is_empty()
                && segment.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
