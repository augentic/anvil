//! Byte-deterministic packing of an adapter directory into a single
//! compressed layer (RFC-48 D1, Shape B).
//!
//! The pack stage walks the adapter tree in sorted order, dereferences
//! symlinks into their real file bytes (so the published artifact is
//! self-contained — RFC-48 D9/D12), and normalizes every tar header
//! (mtime, uid/gid, owner names, permission bits) so the same input
//! tree always produces byte-identical output. The tar stream is then
//! zstd-compressed at a pinned level. The layer's content digest is the
//! immutable, content-addressed adapter identity verified on read
//! (RFC-48 D4).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::error::ExtensionError;

/// Pinned zstd compression level. Fixed so the compressed bytes are
/// reproducible across hosts; never read from the environment.
pub const ZSTD_LEVEL: i32 = 19;

/// Media type of the packed adapter layer in the OCI manifest.
pub const ADAPTER_LAYER_MEDIA_TYPE: &str =
    "application/vnd.augentic.specify.adapter.layer.v1.tar+zstd";

const NORMALIZED_FILE_MODE: u32 = 0o644;
const NORMALIZED_DIR_MODE: u32 = 0o755;

/// Path component names excluded from every pack regardless of the
/// caller's `extra_excluded` list — VCS metadata, CI config, build
/// output, and OS cruft that must never ride inside a published adapter
/// artifact.
const ALWAYS_EXCLUDED: &[&str] = &[".git", ".github", "target", ".DS_Store"];

/// Pack the adapter directory rooted at `root` into a byte-deterministic
/// zstd-compressed tar.
///
/// Symlinks are dereferenced (read through to the target bytes / target
/// directory contents), so the artifact carries shared bundles as plain
/// bytes. `extra_excluded` lists additional path-component names to skip
/// anywhere in the tree (e.g. the declared `extension/` source in the
/// RFC-48 Step 6 build).
///
/// # Errors
///
/// Returns `adapter-pack-failed` when `root` is not a directory, a tree
/// entry cannot be read, a symlink cycle is detected, or the tar / zstd
/// writer fails.
pub fn pack_adapter(root: &Path, extra_excluded: &[&str]) -> Result<Vec<u8>, ExtensionError> {
    if !root.is_dir() {
        return Err(ExtensionError::pack(format!(
            "adapter root {} is not a directory",
            root.display()
        )));
    }

    let mut entries = Vec::new();
    let mut visited = BTreeSet::new();
    collect_entries(root, root, extra_excluded, &mut entries, &mut visited)?;
    // Sort by the on-tape relative path so entry order is independent of
    // the host filesystem's directory iteration order.
    entries.sort_by(|a, b| a.rel.cmp(&b.rel));

    let buffer = Vec::new();
    let encoder = zstd::stream::write::Encoder::new(buffer, ZSTD_LEVEL)
        .map_err(|err| ExtensionError::pack(format!("init zstd encoder: {err}")))?;
    let mut tar = tar::Builder::new(encoder);
    // Force deterministic, normalized headers regardless of source perms.
    tar.mode(tar::HeaderMode::Deterministic);

    for entry in &entries {
        append_entry(&mut tar, entry)?;
    }

    let encoder = tar
        .into_inner()
        .map_err(|err| ExtensionError::pack(format!("finish tar stream: {err}")))?;
    encoder.finish().map_err(|err| ExtensionError::pack(format!("finish zstd stream: {err}")))
}

/// The immutable, content-addressed digest of a packed layer, in the
/// `sha256:<hex>` form OCI references use.
#[must_use]
pub fn content_digest(layer: &[u8]) -> String {
    let mut hasher = specify_schema::digest::Hasher::new();
    hasher.update(layer);
    format!("sha256:{}", hasher.finalize_hex())
}

/// Verify a freshly read layer against the recorded immutable digest
/// (RFC-48 D4 verify-on-read).
///
/// # Errors
///
/// Returns `adapter-digest-mismatch` when the recomputed digest differs
/// from `recorded`.
pub fn verify_digest(reference: &str, layer: &[u8], recorded: &str) -> Result<(), ExtensionError> {
    let actual = content_digest(layer);
    if actual == recorded {
        Ok(())
    } else {
        Err(ExtensionError::digest_mismatch(reference, recorded, actual))
    }
}

/// Unpack a [`pack_adapter`]-produced layer into `dest`.
///
/// Materializes the adapter tree (RFC-48 D5 install into the global
/// store). `dest` is created if absent. Packed layers carry only normal
/// files and directories (symlinks are dereferenced at pack time), so
/// extraction never re-introduces a symlink.
///
/// # Errors
///
/// Returns `adapter-pack-failed` when the layer cannot be zstd-decoded or
/// the tar stream cannot be extracted into `dest`.
pub fn unpack_adapter(layer: &[u8], dest: &Path) -> Result<(), ExtensionError> {
    std::fs::create_dir_all(dest).map_err(|err| {
        ExtensionError::pack(format!("create unpack dir {}: {err}", dest.display()))
    })?;
    let decoder = zstd::stream::read::Decoder::new(layer)
        .map_err(|err| ExtensionError::pack(format!("init zstd decoder: {err}")))?;
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(dest)
        .map_err(|err| ExtensionError::pack(format!("unpack layer into {}: {err}", dest.display())))
}

#[derive(Debug)]
struct PackEntry {
    /// Forward-slash relative path on the tar tape.
    rel: String,
    kind: EntryKind,
}

#[derive(Debug)]
enum EntryKind {
    Dir,
    File(PathBuf),
}

fn collect_entries(
    root: &Path, dir: &Path, extra_excluded: &[&str], out: &mut Vec<PackEntry>,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<(), ExtensionError> {
    // Guard against symlink cycles: a canonical directory is walked once.
    let canonical = std::fs::canonicalize(dir)
        .map_err(|err| ExtensionError::pack(format!("canonicalize {}: {err}", dir.display())))?;
    if !visited.insert(canonical) {
        return Err(ExtensionError::pack(format!(
            "symlink cycle detected while packing {}",
            dir.display()
        )));
    }

    let read = std::fs::read_dir(dir)
        .map_err(|err| ExtensionError::pack(format!("read directory {}: {err}", dir.display())))?;
    let mut children = Vec::new();
    for entry in read {
        let entry = entry.map_err(|err| {
            ExtensionError::pack(format!("read entry in {}: {err}", dir.display()))
        })?;
        children.push(entry.path());
    }

    for path in children {
        let name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => {
                return Err(ExtensionError::pack(format!(
                    "non-UTF-8 path component under {}",
                    dir.display()
                )));
            }
        };
        if ALWAYS_EXCLUDED.contains(&name.as_str()) || extra_excluded.contains(&name.as_str()) {
            continue;
        }
        // `metadata` follows symlinks, so a symlink to a directory is
        // walked as a directory and a symlink to a file is inlined.
        let meta = std::fs::metadata(&path)
            .map_err(|err| ExtensionError::pack(format!("stat {}: {err}", path.display())))?;
        let rel = relative_slash_path(root, &path)?;
        if meta.is_dir() {
            out.push(PackEntry {
                rel,
                kind: EntryKind::Dir,
            });
            collect_entries(root, &path, extra_excluded, out, visited)?;
        } else if meta.is_file() {
            out.push(PackEntry {
                rel,
                kind: EntryKind::File(path),
            });
        } else {
            return Err(ExtensionError::pack(format!(
                "unsupported tree entry {} (not a file or directory)",
                path.display()
            )));
        }
    }
    visited.remove(&std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf()));
    Ok(())
}

fn relative_slash_path(root: &Path, path: &Path) -> Result<String, ExtensionError> {
    let rel = path.strip_prefix(root).map_err(|err| {
        ExtensionError::pack(format!("compute relative path for {}: {err}", path.display()))
    })?;
    let mut parts = Vec::new();
    for component in rel.components() {
        let part = component.as_os_str().to_str().ok_or_else(|| {
            ExtensionError::pack(format!("non-UTF-8 path component in {}", rel.display()))
        })?;
        parts.push(part);
    }
    Ok(parts.join("/"))
}

fn append_entry(
    tar: &mut tar::Builder<zstd::stream::write::Encoder<'static, Vec<u8>>>, entry: &PackEntry,
) -> Result<(), ExtensionError> {
    let mut header = tar::Header::new_gnu();
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    match &entry.kind {
        EntryKind::Dir => {
            header.set_entry_type(tar::EntryType::Directory);
            header.set_mode(NORMALIZED_DIR_MODE);
            header.set_size(0);
            let path = format!("{}/", entry.rel);
            tar.append_data(&mut header, path, std::io::empty())
                .map_err(|err| ExtensionError::pack(format!("append dir {}: {err}", entry.rel)))
        }
        EntryKind::File(source) => {
            let bytes = std::fs::read(source).map_err(|err| {
                ExtensionError::pack(format!("read file {}: {err}", source.display()))
            })?;
            header.set_entry_type(tar::EntryType::Regular);
            header.set_mode(NORMALIZED_FILE_MODE);
            header.set_size(bytes.len() as u64);
            tar.append_data(&mut header, &entry.rel, bytes.as_slice())
                .map_err(|err| ExtensionError::pack(format!("append file {}: {err}", entry.rel)))
        }
    }
}

#[cfg(test)]
mod tests;
