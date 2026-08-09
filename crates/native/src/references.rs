//! Native reference hosting: per-adapter MCP shelves at `/mcp/<name>`
//! plus the shared lazy loopback listener. Without the `cli` feature
//! an online provider fails loudly on the first reference-carrying op.

use adapter::references::References;
use omnia_guest::axum::Router;

use crate::catalog::Catalog;

/// One mounted reference shelf.
#[derive(Clone, Copy, Debug)]
pub struct Shelf {
    /// Axis-stripped adapter name.
    pub name: &'static str,
    /// The adapter's references server over its embedded registry.
    pub references: References,
}

/// Every linked adapter's reference shelf.
///
/// Shelf identity uses the catalog entry's name and version, not the
/// `native` crate version.
#[must_use]
pub fn shelves(catalog: &Catalog) -> Vec<Shelf> {
    catalog
        .entries()
        .iter()
        .map(|entry| Shelf {
            name: entry.name(),
            references: References {
                server_name: entry.server_name(),
                version: entry.version(),
                docs: entry.docs(),
            },
        })
        .collect()
}

/// Shelf router nested at `/mcp/<name>`, ready to serve or to merge
/// onto an embedding host's router.
///
/// A linked implementor registered on both axes (legal for linked
/// crates, unlike component exports) shares one embedded docs
/// registry, so its shelf mounts once.
pub fn router(catalog: &Catalog) -> Router {
    let mut mounted = std::collections::HashSet::new();
    shelves(catalog).into_iter().filter(|shelf| mounted.insert(shelf.name)).fold(
        Router::new(),
        |router, shelf| {
            router.nest(&format!("/mcp/{}", shelf.name), omnia_guest::mcp::router(shelf.references))
        },
    )
}

/// Whether any linked adapter carries reference documents; a
/// document-free catalog keeps online reference hosting a no-op.
#[must_use]
pub fn has_docs(catalog: &Catalog) -> bool {
    catalog.entries().iter().any(|entry| !entry.docs().is_empty())
}

#[cfg(feature = "cli")]
pub use host::ReferenceHost;

#[cfg(feature = "cli")]
mod host {
    use std::sync::Mutex;

    use tokio::sync::OnceCell;

    use crate::catalog::Catalog;
    use crate::error::Error;

    /// The owned lazy loopback listener one online provider graph
    /// shares.
    ///
    /// The listener binds only to `127.0.0.1`, starts on the first
    /// request for a shelf URL, and is shared by every provider clone.
    /// Command execution requests graceful shutdown and awaits the
    /// server task on every exit path; [`Drop`] retains an abort
    /// fallback for embedders that never call [`Self::shutdown`].
    #[derive(Debug)]
    pub struct ReferenceHost {
        catalog: Catalog,
        started: OnceCell<Started>,
    }

    #[derive(Debug)]
    struct Started {
        base: String,
        stop: tokio::sync::watch::Sender<bool>,
        task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    }

    impl ReferenceHost {
        /// A dormant host over `catalog`'s shelves.
        #[must_use]
        pub const fn new(catalog: Catalog) -> Self {
            Self {
                catalog,
                started: OnceCell::const_new(),
            }
        }

        /// The shelf base URL, starting the shared listener on first
        /// use. Returns `Ok(None)` when the catalog carries no
        /// reference documents.
        ///
        /// # Errors
        ///
        /// [`Error::Listener`] when the loopback bind fails; the
        /// failure fails the requesting operation rather than
        /// stripping its grants.
        pub async fn base(&self) -> Result<Option<String>, Error> {
            if !super::has_docs(&self.catalog) {
                return Ok(None);
            }
            let started = self.started.get_or_try_init(|| self.start()).await?;
            Ok(Some(started.base.clone()))
        }

        async fn start(&self) -> Result<Started, Error> {
            let listener =
                tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.map_err(|err| {
                    Error::Listener {
                        detail: format!("binding the loopback reference listener: {err}"),
                    }
                })?;
            let port = listener
                .local_addr()
                .map_err(|err| Error::Listener {
                    detail: format!("reading the reference listener address: {err}"),
                })?
                .port();
            let (stop, mut stopped) = tokio::sync::watch::channel(false);
            let router = super::router(&self.catalog);
            let task = tokio::spawn(async move {
                let shutdown = async move {
                    drop(stopped.wait_for(|stop| *stop).await);
                };
                drop(axum::serve(listener, router).with_graceful_shutdown(shutdown).await);
            });
            Ok(Started {
                base: format!("http://127.0.0.1:{port}"),
                stop,
                task: Mutex::new(Some(task)),
            })
        }

        /// Request graceful shutdown and await the server task. A
        /// never-started or already-stopped host is a no-op.
        ///
        /// # Panics
        ///
        /// Panics when the server-task lock is poisoned (never in
        /// practice).
        pub async fn shutdown(&self) {
            let Some(started) = self.started.get() else {
                return;
            };
            let _ = started.stop.send(true);
            let task = started.task.lock().expect("reference task lock").take();
            if let Some(task) = task {
                drop(task.await);
            }
        }
    }

    impl Drop for ReferenceHost {
        fn drop(&mut self) {
            // Abort fallback for embedders that never awaited shutdown.
            if let Some(started) = self.started.get()
                && let Ok(mut task) = started.task.lock()
                && let Some(task) = task.take()
            {
                task.abort();
            }
        }
    }
}
