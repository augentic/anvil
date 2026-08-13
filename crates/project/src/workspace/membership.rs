//! Product-ignore membership for the snapshot walks (RFC-105): the
//! tree's own `.gitignore` files filter admission after the kernel
//! excludes, which win — a negation can never re-admit them.

use std::path::Path;
use std::sync::Arc;

use ignore::Match;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

/// The `.gitignore` file name read at each directory level.
const GITIGNORE: &str = ".gitignore";

/// The gitignore matcher stack for one directory level of a walk.
///
/// Built only from `.gitignore` files in the tree being walked —
/// operator git state (`info/exclude`, `core.excludesfile`, the
/// index) is never consulted. Cheap to clone (shared matchers), so
/// iterative walks carry one per pending directory and recursive
/// walks pass it down by reference.
#[derive(Clone, Debug, Default)]
pub struct Ignores {
    /// Outermost-first matcher stack; matching consults it innermost
    /// first, mirroring git's deeper-file-wins precedence.
    stack: Vec<Arc<Gitignore>>,
}

impl Ignores {
    /// The stack extended by `dir`'s `.gitignore`, when present.
    ///
    /// Call once per directory before walking its entries — including
    /// the walk root, whose `.gitignore` applies to everything below.
    /// Unparseable patterns are dropped, matching git's tolerance of
    /// lines it cannot use; the parseable remainder still applies.
    #[must_use]
    pub fn descend(&self, dir: &Path) -> Self {
        let file = dir.join(GITIGNORE);
        if !file.is_file() {
            return self.clone();
        }
        let mut builder = GitignoreBuilder::new(dir);
        drop(builder.add(&file));
        let Ok(matcher) = builder.build() else {
            return self.clone();
        };
        let mut stack = self.stack.clone();
        stack.push(Arc::new(matcher));
        Self { stack }
    }

    /// Whether gitignore rules exclude `path` (`is_dir` selects git's
    /// trailing-slash directory semantics; a symlink is not a
    /// directory). The innermost matcher with an opinion wins; a
    /// whitelist (`!pattern`) match re-admits.
    pub fn excluded(&self, path: &Path, is_dir: bool) -> bool {
        for matcher in self.stack.iter().rev() {
            match matcher.matched(path, is_dir) {
                Match::None => {}
                Match::Ignore(_) => return true,
                Match::Whitelist(_) => return false,
            }
        }
        false
    }
}
