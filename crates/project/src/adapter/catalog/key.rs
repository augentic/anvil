//! Source-key assignment: basename, reserved `intent`, digest suffixes.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::Error;

use super::{INTENT, Pin};
use crate::binding::{Location, Locator, Origin};

/// One source row waiting for a key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// Location or inline value.
    pub origin: Origin,
    /// Exact adapter pin already selected for this row.
    pub pin: Pin,
}

/// Assign source keys for `rows`.
///
/// `prior` maps a row identity to a previously persisted key so
/// unchanged bindings keep their keys. New collisions receive a
/// stable digest suffix; duplicate identities fail.
///
/// # Errors
///
/// `source-adapter-duplicate` when two rows share an identity;
/// `source-intent-locator` when `intent` carries a locator.
pub fn assign(rows: &[Row], prior: &BTreeMap<String, String>) -> Result<Vec<String>, Error> {
    let mut seen = BTreeSet::new();
    let mut identities = Vec::with_capacity(rows.len());
    for row in rows {
        refuse_intent_locator(row)?;
        let id = identity(row);
        if !seen.insert(id.clone()) {
            return Err(Error::Diag {
                code: "source-adapter-duplicate",
                detail: format!("duplicate source binding `{id}`"),
            });
        }
        identities.push(id);
    }

    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mut keys = vec![String::new(); rows.len()];
    let mut pending = Vec::new();
    for (index, id) in identities.iter().enumerate() {
        if let Some(key) = prior.get(id) {
            keys[index].clone_from(key);
            taken.insert(key.clone());
        } else {
            pending.push(index);
        }
    }

    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for index in pending {
        let proposed = proposed(&rows[index]);
        groups.entry(proposed).or_default().push(index);
    }

    for (base, members) in groups {
        let mut members = members;
        members.sort_by(|&left, &right| identities[left].cmp(&identities[right]));
        for (rank, index) in members.into_iter().enumerate() {
            let intent_ok = base != INTENT || rows[index].pin.name == INTENT;
            let key = if rank == 0 && intent_ok && !taken.contains(&base) {
                base.clone()
            } else {
                unique(&base, &identities[index], &taken)
            };
            taken.insert(key.clone());
            keys[index] = key;
        }
    }
    Ok(keys)
}

/// Stable identity for duplicate detection and prior-key lookup.
#[must_use]
pub fn identity(row: &Row) -> String {
    if row.pin.name == INTENT {
        return INTENT.to_string();
    }
    match &row.origin {
        Origin::Location(location) => location.key(),
        Origin::Value(value) => format!("value:{}:{}", row.pin.name, sha256_hex(value.as_bytes())),
    }
}

fn proposed(row: &Row) -> String {
    if row.pin.name == INTENT {
        return INTENT.to_string();
    }
    match &row.origin {
        Origin::Location(location) => kebab(&basename(location)),
        Origin::Value(_) => kebab(&row.pin.name),
    }
}

fn refuse_intent_locator(row: &Row) -> Result<(), Error> {
    if row.pin.name == INTENT && matches!(row.origin, Origin::Location(_)) {
        return Err(intent_locator());
    }
    Ok(())
}

pub(super) fn intent_locator() -> Error {
    Error::Diag {
        code: "source-intent-locator",
        detail: "adapter `intent` is inline `value` only; a locator is refused".into(),
    }
}

fn basename(location: &Location) -> String {
    if location.path != "." {
        return file_stem(Path::new(&location.path));
    }
    match &location.locator {
        Locator::Git { url, .. } => file_stem(Path::new(url)),
        Locator::Path(path) => file_stem(path),
        Locator::Https(url) => {
            let trimmed = url.split('?').next().unwrap_or(url);
            let trimmed = trimmed.split('#').next().unwrap_or(trimmed);
            let trimmed = trimmed.trim_end_matches('/');
            file_stem(Path::new(trimmed))
        }
    }
}

fn file_stem(path: &Path) -> String {
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
    let name = name.strip_suffix(".git").unwrap_or(name);
    Path::new(name).file_stem().and_then(|stem| stem.to_str()).unwrap_or(name).to_string()
}

fn kebab(raw: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in raw.chars() {
        let mapped = match ch {
            'A'..='Z' => Some(ch.to_ascii_lowercase()),
            'a'..='z' | '0'..='9' => Some(ch),
            _ => None,
        };
        if let Some(ch) = mapped {
            out.push(ch);
            dash = false;
        } else if !out.is_empty() && !dash {
            out.push('-');
            dash = true;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { "source".into() } else { out }
}

fn unique(base: &str, identity: &str, taken: &BTreeSet<String>) -> String {
    let digest = sha256_hex(identity.as_bytes());
    let stem = if base == INTENT { "source" } else { base };
    for width in 8..=16 {
        let key = format!("{stem}-{}", &digest[..width]);
        if !taken.contains(&key) {
            return key;
        }
    }
    format!("{stem}-{digest}")
}
