//! Model-capability profiles: digest, scoring kernel, host table.

use std::collections::BTreeMap;
use std::path::PathBuf;

use project::profile::{
    Assessment, FRONTIER_LARGE, FRONTIER_LARGE_V1, Gate, Profile, Table, VERSION,
};

fn code(err: &impl std::fmt::Display) -> String {
    err.to_string()
}

const fn dims(breadth: u8, coupling: u8, uncertainty: u8, volume: u8, surface: u8) -> Assessment {
    Assessment {
        behavioural_breadth: breadth,
        coupling,
        uncertainty,
        context_volume: volume,
        verification_surface: surface,
    }
}

mod digest {
    use super::*;

    fn golden_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("answers").join("capability-profile.yaml")
    }

    #[test]
    fn compiled_body_golden() {
        let actual = Profile::frontier_v1().canonical_yaml().expect("yaml");
        let path = golden_path();
        if std::env::var_os("REGENERATE_GOLDENS").is_some() {
            std::fs::create_dir_all(path.parent().expect("answers dir")).expect("create");
            std::fs::write(&path, &actual).expect("regenerate");
        }
        let expected = std::fs::read_to_string(&path).expect("read golden");
        assert_eq!(actual, expected, "golden mismatch: {}", path.display());
        let digest = Profile::frontier_v1().digest().expect("digest");
        let again = Profile::parse(&expected).expect("parse golden").digest().expect("re-digest");
        assert_eq!(digest, again);
    }

    #[test]
    fn unknown_field_rejected() {
        let mut yaml = Profile::frontier_v1().canonical_yaml().expect("yaml");
        yaml.push_str("extra: true\n");
        let err = Profile::parse(&yaml).expect_err("unknown");
        assert!(code(&err).contains("profile-malformed"), "{err}");
    }

    #[test]
    fn version_and_id() {
        let mut profile = Profile::frontier_v1();
        profile.version = VERSION + 1;
        let err = Profile::parse(&profile.canonical_yaml().expect("yaml")).expect_err("version");
        assert!(code(&err).contains("profile-version"), "{err}");

        profile = Profile::frontier_v1();
        profile.id.clear();
        let err = Profile::parse(&profile.canonical_yaml().expect("yaml")).expect_err("id");
        assert!(code(&err).contains("profile-malformed"), "{err}");
    }
}

mod score {
    use super::*;

    #[test]
    fn weighted_sum_and_gates() {
        let profile = Profile::frontier_v1();
        // zeros → 0: below both thresholds.
        let zero = dims(0, 0, 0, 0, 0);
        assert_eq!(profile.score(&zero).expect("score"), 0);
        assert!(!profile.exceeds(&zero, Gate::SliceSplit).expect("gate"));
        assert!(!profile.exceeds(&zero, Gate::Task).expect("gate"));

        // all tens → 10*(3+4+2+1+3) = 130: above both.
        let full = dims(10, 10, 10, 10, 10);
        assert_eq!(profile.score(&full).expect("score"), 130);
        assert!(profile.exceeds(&full, Gate::SliceSplit).expect("gate"));
        assert!(profile.exceeds(&full, Gate::Task).expect("gate"));

        // 10*3 + 10*4 + 5*2 = 80 exactly: at slice-split, above task.
        let at_split = dims(10, 10, 5, 0, 0);
        assert_eq!(profile.score(&at_split).expect("score"), 80);
        assert!(!profile.exceeds(&at_split, Gate::SliceSplit).expect("gate"));
        assert!(profile.exceeds(&at_split, Gate::Task).expect("gate"));

        // one more uncertainty point → 82: above both.
        let over = dims(10, 10, 6, 0, 0);
        assert_eq!(profile.score(&over).expect("score"), 82);
        assert!(profile.exceeds(&over, Gate::SliceSplit).expect("gate"));

        // 8*4 + 3*1 = 35 exactly: at task, below slice-split.
        let at_task = dims(0, 8, 0, 3, 0);
        assert_eq!(profile.score(&at_task).expect("score"), 35);
        assert!(!profile.exceeds(&at_task, Gate::Task).expect("gate"));
        assert!(!profile.exceeds(&at_task, Gate::SliceSplit).expect("gate"));
    }

    #[test]
    fn dimension_range() {
        let profile = Profile::frontier_v1();
        let err = profile.score(&dims(11, 0, 0, 0, 0)).expect_err("range");
        assert!(code(&err).contains("profile-dimension-range"), "{err}");
        assert!(code(&err).contains("behavioural-breadth"), "{err}");
    }

    #[test]
    fn overflow() {
        let mut profile = Profile::frontier_v1();
        profile.weights.behavioural_breadth = u32::MAX;
        profile.weights.coupling = u32::MAX;
        let err = profile.score(&dims(10, 10, 0, 0, 0)).expect_err("overflow");
        assert!(code(&err).contains("profile-score-overflow"), "{err}");
    }
}

mod table {
    use super::*;

    #[test]
    fn compiled_resolve() {
        let table = Table::compiled();
        let profile = table.resolve().expect("sole entry");
        assert_eq!(profile.id, FRONTIER_LARGE_V1);
        assert_eq!(table.get(FRONTIER_LARGE).expect("class").id, FRONTIER_LARGE_V1);
        let err = table.get("absent").expect_err("missing");
        assert!(code(&err).contains("profile-class-unknown"), "{err}");
    }

    #[test]
    fn host_replace_digest() {
        let compiled = Table::compiled().resolve().expect("compiled").digest().expect("digest");
        let mut other = Profile::frontier_v1();
        other.weights.coupling = 5;
        let table = Table::new(BTreeMap::from([(FRONTIER_LARGE.into(), other)])).expect("table");
        let digest = table.resolve().expect("override").digest().expect("digest");
        assert_ne!(digest, compiled);
        assert_eq!(table.resolve().expect("id").id, FRONTIER_LARGE_V1);
    }

    #[test]
    fn refusals() {
        let err = Table::new(BTreeMap::new()).expect_err("empty");
        assert!(code(&err).contains("profile-table-invalid"), "{err}");

        let err = Table::new(BTreeMap::from([(String::new(), Profile::frontier_v1())]))
            .expect_err("empty class");
        assert!(code(&err).contains("profile-table-invalid"), "{err}");

        let mut second = Profile::frontier_v1();
        second.id = "other-v1".into();
        let dup_id = Profile::frontier_v1();
        let err = Table::new(BTreeMap::from([("a".into(), dup_id.clone()), ("b".into(), dup_id)]))
            .expect_err("dup id");
        assert!(code(&err).contains("profile-table-invalid"), "{err}");

        let many = Table::new(BTreeMap::from([
            (FRONTIER_LARGE.into(), Profile::frontier_v1()),
            ("other".into(), second),
        ]))
        .expect("two classes");
        let err = many.resolve().expect_err("need class");
        assert!(code(&err).contains("profile-class-required"), "{err}");
        assert_eq!(many.get("other").expect("get").id, "other-v1");
    }
}
