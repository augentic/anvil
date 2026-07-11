//! Integration coverage for the closed [`Platform`] enum and the
//! `--platforms` CSV parser. The reject branch (unknown token) is
//! covered end-to-end by `tests/init.rs` at the repo root; the
//! wire-name coherence and the CSV sort/dedup/whitespace accept-half
//! live here against the public surface.

use workflow::{Platform, parse_platforms_csv};

#[test]
fn wire_names_and_csv() {
    // The strum-derived `Display` / `FromStr` (`serialize_all =
    // "kebab-case"`) must not drift from the serde
    // `#[serde(rename_all = "kebab-case")]` wire name.
    for platform in
        [Platform::Core, Platform::Ios, Platform::Android, Platform::Web, Platform::Desktop]
    {
        let name = platform.to_string();
        assert_eq!(name.parse::<Platform>().unwrap(), platform, "FromStr(Display) round trip");
        let yaml = serde_saphyr::to_string(&platform).unwrap();
        assert_eq!(yaml.trim(), name, "serde wire name must match Display");
    }

    // Whitespace and empty tokens are tolerated; duplicates collapse;
    // output is sorted with `Core` first; the first unknown token is
    // named in the error.
    let platforms = parse_platforms_csv(" android , core ,ios,core,").unwrap();
    assert_eq!(platforms, vec![Platform::Core, Platform::Ios, Platform::Android]);

    let err = parse_platforms_csv("core,windows").unwrap_err();
    assert!(err.contains("windows"), "error should name the bad token: {err}");
}
