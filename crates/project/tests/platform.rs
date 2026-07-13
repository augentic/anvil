//! Provider-contract coverage for the closed [`Platform`] enum.

use workflow::Platform;

#[test]
fn wire_names() {
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
}
