//! Scripted in-memory storage for the native suites: tests observe
//! engine state through this provider — mutex-held maps with
//! inspection helpers — instead of the filesystem. Shared across the
//! journey, home, and wire-contract suites via `#[path]` inclusion.

#![expect(dead_code, reason = "shared scripted storage; each consuming binary uses a subset")]

use std::collections::BTreeMap;
use std::future::{Future, ready};
use std::sync::Mutex;

use omnia_guest::{BlobStore, CasError, ContainerMetadata, ObjectMetadata, StateStore};

type State = BTreeMap<String, Vec<u8>>;
type Blobs = BTreeMap<String, BTreeMap<String, Vec<u8>>>;

/// In-memory keyvalue + blobstore scripting the storage seam.
#[derive(Debug, Default)]
pub struct Memory {
    state: Mutex<State>,
    blobs: Mutex<Blobs>,
}

impl Memory {
    /// The stored keyvalue entry at `key`.
    pub fn state(&self, key: &str) -> Option<Vec<u8>> {
        self.state.lock().expect("state lock").get(key).cloned()
    }

    /// The stored object bytes at `container`/`name`.
    pub fn object(&self, container: &str, name: &str) -> Option<Vec<u8>> {
        self.blobs.lock().expect("blob lock").get(container)?.get(name).cloned()
    }

    /// Every object name in `container`, sorted.
    pub fn objects(&self, container: &str) -> Vec<String> {
        self.blobs
            .lock()
            .expect("blob lock")
            .get(container)
            .map(|objects| objects.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Seed a keyvalue entry without going through the seam.
    pub fn insert_state(&self, key: &str, bytes: &[u8]) {
        drop(self.state.lock().expect("state lock").insert(key.to_string(), bytes.to_vec()));
    }

    /// Seed an object without going through the seam.
    pub fn insert_object(&self, container: &str, name: &str, bytes: &[u8]) {
        drop(
            self.blobs
                .lock()
                .expect("blob lock")
                .entry(container.to_string())
                .or_default()
                .insert(name.to_string(), bytes.to_vec()),
        );
    }

    /// True when nothing has ever been stored.
    pub fn is_empty(&self) -> bool {
        self.state.lock().expect("state lock").is_empty()
            && self.blobs.lock().expect("blob lock").values().all(BTreeMap::is_empty)
    }

    /// The full store contents, for byte-stability comparisons.
    pub fn snapshot(&self) -> (State, Blobs) {
        (
            self.state.lock().expect("state lock").clone(),
            self.blobs.lock().expect("blob lock").clone(),
        )
    }
}

fn unscripted<T: Send>(operation: &str) -> impl Future<Output = anyhow::Result<T>> + Send + use<T> {
    ready(Err(anyhow::anyhow!("{operation} is not scripted")))
}

impl StateStore for Memory {
    fn get(&self, key: &str) -> impl Future<Output = anyhow::Result<Option<Vec<u8>>>> + Send {
        ready(Ok(self.state(key)))
    }

    fn set(
        &self, key: &str, value: &[u8], ttl_secs: Option<u64>,
    ) -> impl Future<Output = anyhow::Result<Option<Vec<u8>>>> + Send {
        assert!(ttl_secs.is_none(), "the engine never sets a TTL");
        let previous =
            self.state.lock().expect("state lock").insert(key.to_string(), value.to_vec());
        ready(Ok(previous))
    }

    fn delete(&self, key: &str) -> impl Future<Output = anyhow::Result<()>> + Send {
        drop(self.state.lock().expect("state lock").remove(key));
        ready(Ok(()))
    }

    fn cas(
        &self, key: &str, expected: Option<&[u8]>, value: &[u8],
    ) -> impl Future<Output = Result<(), CasError>> + Send {
        let mut state = self.state.lock().expect("state lock");
        let observed = state.get(key).cloned();
        let swapped = if observed.as_deref() == expected {
            drop(state.insert(key.to_string(), value.to_vec()));
            Ok(())
        } else {
            Err(CasError::Conflict(observed))
        };
        drop(state);
        ready(swapped)
    }

    fn increment(
        &self, _key: &str, _delta: i64,
    ) -> impl Future<Output = anyhow::Result<i64>> + Send {
        unscripted("increment")
    }
}

impl BlobStore for Memory {
    fn get(
        &self, container: &str, name: &str,
    ) -> impl Future<Output = anyhow::Result<Option<Vec<u8>>>> + Send {
        ready(Ok(self.object(container, name)))
    }

    fn put(
        &self, container: &str, name: &str, data: &[u8],
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        self.insert_object(container, name, data);
        ready(Ok(()))
    }

    fn delete(
        &self, container: &str, name: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        if let Some(objects) = self.blobs.lock().expect("blob lock").get_mut(container) {
            drop(objects.remove(name));
        }
        ready(Ok(()))
    }

    fn has(
        &self, container: &str, name: &str,
    ) -> impl Future<Output = anyhow::Result<bool>> + Send {
        ready(Ok(self.object(container, name).is_some()))
    }

    fn list(&self, container: &str) -> impl Future<Output = anyhow::Result<Vec<String>>> + Send {
        ready(Ok(self.objects(container)))
    }

    fn get_range(
        &self, _container: &str, _name: &str, _start: u64, _end: u64,
    ) -> impl Future<Output = anyhow::Result<Vec<u8>>> + Send {
        unscripted("get_range")
    }

    fn object_info(
        &self, _container: &str, _name: &str,
    ) -> impl Future<Output = anyhow::Result<ObjectMetadata>> + Send {
        unscripted("object_info")
    }

    fn delete_objects(
        &self, _container: &str, _names: &[String],
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        unscripted("delete_objects")
    }

    fn clear(&self, _container: &str) -> impl Future<Output = anyhow::Result<()>> + Send {
        unscripted("clear")
    }

    fn create_container(&self, _name: &str) -> impl Future<Output = anyhow::Result<()>> + Send {
        unscripted("create_container")
    }

    fn delete_container(&self, _name: &str) -> impl Future<Output = anyhow::Result<()>> + Send {
        unscripted("delete_container")
    }

    fn container_exists(&self, _name: &str) -> impl Future<Output = anyhow::Result<bool>> + Send {
        unscripted("container_exists")
    }

    fn container_info(
        &self, _container: &str,
    ) -> impl Future<Output = anyhow::Result<ContainerMetadata>> + Send {
        unscripted("container_info")
    }

    fn copy_object(
        &self, _src_container: &str, _src_name: &str, _dest_container: &str, _dest_name: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        unscripted("copy_object")
    }

    fn move_object(
        &self, _src_container: &str, _src_name: &str, _dest_container: &str, _dest_name: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        unscripted("move_object")
    }
}

/// Implement both storage capabilities on a provider by forwarding to
/// its shared [`Memory`] field.
#[macro_export]
macro_rules! scripted_storage {
    ($provider:ty, $field:ident) => {
        impl omnia_guest::StateStore for $provider {
            fn get(
                &self, key: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<Option<Vec<u8>>>> + Send {
                omnia_guest::StateStore::get(&*self.$field, key)
            }

            fn set(
                &self, key: &str, value: &[u8], ttl_secs: Option<u64>,
            ) -> impl ::std::future::Future<Output = anyhow::Result<Option<Vec<u8>>>> + Send {
                omnia_guest::StateStore::set(&*self.$field, key, value, ttl_secs)
            }

            fn delete(
                &self, key: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<()>> + Send {
                omnia_guest::StateStore::delete(&*self.$field, key)
            }

            fn cas(
                &self, key: &str, expected: Option<&[u8]>, value: &[u8],
            ) -> impl ::std::future::Future<Output = Result<(), omnia_guest::CasError>> + Send
            {
                omnia_guest::StateStore::cas(&*self.$field, key, expected, value)
            }

            fn increment(
                &self, key: &str, delta: i64,
            ) -> impl ::std::future::Future<Output = anyhow::Result<i64>> + Send {
                omnia_guest::StateStore::increment(&*self.$field, key, delta)
            }
        }

        impl omnia_guest::BlobStore for $provider {
            fn get(
                &self, container: &str, name: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<Option<Vec<u8>>>> + Send {
                omnia_guest::BlobStore::get(&*self.$field, container, name)
            }

            fn put(
                &self, container: &str, name: &str, data: &[u8],
            ) -> impl ::std::future::Future<Output = anyhow::Result<()>> + Send {
                omnia_guest::BlobStore::put(&*self.$field, container, name, data)
            }

            fn delete(
                &self, container: &str, name: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<()>> + Send {
                omnia_guest::BlobStore::delete(&*self.$field, container, name)
            }

            fn has(
                &self, container: &str, name: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<bool>> + Send {
                omnia_guest::BlobStore::has(&*self.$field, container, name)
            }

            fn list(
                &self, container: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<Vec<String>>> + Send {
                omnia_guest::BlobStore::list(&*self.$field, container)
            }

            fn get_range(
                &self, container: &str, name: &str, start: u64, end: u64,
            ) -> impl ::std::future::Future<Output = anyhow::Result<Vec<u8>>> + Send {
                omnia_guest::BlobStore::get_range(&*self.$field, container, name, start, end)
            }

            fn object_info(
                &self, container: &str, name: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<omnia_guest::ObjectMetadata>> + Send
            {
                omnia_guest::BlobStore::object_info(&*self.$field, container, name)
            }

            fn delete_objects(
                &self, container: &str, names: &[String],
            ) -> impl ::std::future::Future<Output = anyhow::Result<()>> + Send {
                omnia_guest::BlobStore::delete_objects(&*self.$field, container, names)
            }

            fn clear(
                &self, container: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<()>> + Send {
                omnia_guest::BlobStore::clear(&*self.$field, container)
            }

            fn create_container(
                &self, name: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<()>> + Send {
                omnia_guest::BlobStore::create_container(&*self.$field, name)
            }

            fn delete_container(
                &self, name: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<()>> + Send {
                omnia_guest::BlobStore::delete_container(&*self.$field, name)
            }

            fn container_exists(
                &self, name: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<bool>> + Send {
                omnia_guest::BlobStore::container_exists(&*self.$field, name)
            }

            fn container_info(
                &self, container: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<omnia_guest::ContainerMetadata>> + Send
            {
                omnia_guest::BlobStore::container_info(&*self.$field, container)
            }

            fn copy_object(
                &self, src_container: &str, src_name: &str, dest_container: &str, dest_name: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<()>> + Send {
                omnia_guest::BlobStore::copy_object(
                    &*self.$field,
                    src_container,
                    src_name,
                    dest_container,
                    dest_name,
                )
            }

            fn move_object(
                &self, src_container: &str, src_name: &str, dest_container: &str, dest_name: &str,
            ) -> impl ::std::future::Future<Output = anyhow::Result<()>> + Send {
                omnia_guest::BlobStore::move_object(
                    &*self.$field,
                    src_container,
                    src_name,
                    dest_container,
                    dest_name,
                )
            }
        }
    };
}
