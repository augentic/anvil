//! Per-target accepted-CID projection from `target.merge.wave-committed`
//! facts. A broken `{base, result}` chain is a typed refusal.

use error::Error;

use super::Wave;
use crate::config::Layout;
use crate::journal::{Event, EventKind};
use crate::snapshot::SnapshotId;

/// Current accepted CID for `target`, or `None` when no wave has
/// opened yet (the first wave freezes ambient).
///
/// The chain starts at the first committed fact's `base` (the freeze
/// taken when that target's first wave opened) and walks `{base,
/// result}` transitions. With no commits yet, the first opened wave's
/// recorded `base` is the accepted CID.
///
/// # Errors
///
/// `target-accepted-cid-broken-chain` when a committed fact's `base`
/// is not the prior accepted CID; wave-manifest load failures.
pub fn accepted_cid(
    layout: Layout<'_>, events: &[Event], target: &str,
) -> Result<Option<SnapshotId>, Error> {
    let commits: Vec<(&SnapshotId, &SnapshotId)> = events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::TargetMergeWaveCommitted {
                target: committed,
                base,
                result,
                ..
            } if committed == target => Some((base, result)),
            _ => None,
        })
        .collect();

    if let Some((first_base, _)) = commits.first() {
        let mut current = (*first_base).clone();
        for (base, result) in &commits {
            if *base != &current {
                return Err(Error::Diag {
                    code: "target-accepted-cid-broken-chain",
                    detail: format!(
                        "target `{target}` accepted-CID chain broke: committed fact base `{base}` \
                         is not the prior accepted CID `{current}`"
                    ),
                });
            }
            current = (*result).clone();
        }
        return Ok(Some(current));
    }

    initial_cid(layout, events, target)
}

/// Base recorded on the target's first opened wave — the in-place
/// initial CID — or `None` when no wave has opened.
fn initial_cid(
    layout: Layout<'_>, events: &[Event], target: &str,
) -> Result<Option<SnapshotId>, Error> {
    let Some(digest) = events.iter().find_map(|event| match &event.kind {
        EventKind::TargetWaveOpened {
            target: opened,
            digest,
            ..
        } if opened == target => Some(digest.as_str()),
        _ => None,
    }) else {
        return Ok(None);
    };
    Ok(Some(Wave::load(layout, target, digest)?.base))
}
