use strum::VariantArray;

use super::*;

/// Legal directed edges, mirroring the `matches!` arm in
/// [`LifecycleStatus::transition`]. The matrix test treats every other
/// `(from, to)` pair over [`LifecycleStatus::VARIANTS`] as illegal.
const LEGAL_EDGES: &[(LifecycleStatus, LifecycleStatus)] = &[
    (LifecycleStatus::Refining, LifecycleStatus::Refined),
    (LifecycleStatus::Refined, LifecycleStatus::Built),
    (LifecycleStatus::Built, LifecycleStatus::Merged),
    (LifecycleStatus::Refining, LifecycleStatus::Dropped),
    (LifecycleStatus::Refined, LifecycleStatus::Dropped),
    (LifecycleStatus::Built, LifecycleStatus::Dropped),
];

#[test]
fn transition_refining_to_refined_ok() {
    assert_eq!(
        LifecycleStatus::Refining.transition(LifecycleStatus::Refined).expect("legal"),
        LifecycleStatus::Refined,
    );
}

#[test]
fn transition_rejects_skipping_states() {
    let Err(Error::Diag { code, detail }) =
        LifecycleStatus::Refining.transition(LifecycleStatus::Built)
    else {
        panic!("Refining -> Built skips Refined; must Err with lifecycle diag");
    };
    assert_eq!(code, "lifecycle");
    assert!(detail.contains("Refining") && detail.contains("Built"), "endpoints in: {detail:?}");
}

#[test]
fn transition_matrix_legal_and_illegal() {
    // Cartesian product over every state pair; self-transitions included.
    // Legality is derived from `LEGAL_EDGES`, not re-asserted per arm, so the
    // table stays the single source of truth alongside the machine itself.
    let states = LifecycleStatus::VARIANTS;
    for &from in states {
        for &to in states {
            let legal = LEGAL_EDGES.contains(&(from, to));
            match from.transition(to) {
                Ok(next) => {
                    assert!(legal, "{from:?} -> {to:?} succeeded but is not a legal edge");
                    assert_eq!(next, to, "{from:?} -> {to:?} must yield the target state");
                }
                Err(Error::Diag { code, detail }) => {
                    assert!(!legal, "{from:?} -> {to:?} is legal but was rejected");
                    assert_eq!(code, "lifecycle", "{from:?} -> {to:?} reject code");
                    assert!(
                        detail.contains(&format!("{from:?}"))
                            && detail.contains(&format!("{to:?}")),
                        "{from:?} -> {to:?} endpoints in detail: {detail:?}",
                    );
                }
                Err(other) => panic!("{from:?} -> {to:?} unexpected error: {other:?}"),
            }
        }
    }
}
