use super::*;
use crate::lint::FileKind;

fn skill_file(relative: &str, body: &str) -> DiscoveredFile {
    DiscoveredFile {
        relative: relative.into(),
        kind: FileKind::Text,
        language: Some("markdown".into()),
        bytes: Some(body.as_bytes().to_vec()),
    }
}

#[test]
fn extracts_skill_from_well_formed_path() {
    let file = skill_file(
        "plugins/spec/skills/init/SKILL.md",
        "---\nname: specify-init\ndescription: Init skill.\n---\n# Body\n\nLine two.\n",
    );
    let skill = extract(&file).expect("skill extracted");
    assert_eq!(skill.name, "specify-init");
    assert_eq!(skill.plugin, "spec");
    assert_eq!(skill.path, "plugins/spec/skills/init/SKILL.md");
    assert_eq!(skill.frontmatter_ref, "plugins/spec/skills/init/SKILL.md#frontmatter");
    assert!(skill.body_line_count.unwrap() >= 1);
}

#[test]
fn rejects_non_skill_paths() {
    let file = skill_file("plugins/spec/SKILL.md", "---\nname: nope\n---\n");
    assert!(extract(&file).is_none());
}

#[test]
fn rejects_skill_without_name() {
    let file =
        skill_file("plugins/spec/skills/init/SKILL.md", "---\ndescription: missing name\n---\n");
    assert!(extract(&file).is_none());
}
