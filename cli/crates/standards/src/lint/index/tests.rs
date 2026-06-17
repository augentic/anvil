use super::*;

#[test]
fn url_scheme_accepts_common() {
    assert!(is_url_scheme("https://example.com"));
    assert!(is_url_scheme("http://example.com"));
    assert!(is_url_scheme("mailto://x"));
    assert!(is_url_scheme("file://something"));
    assert!(!is_url_scheme("./local.md"));
    assert!(!is_url_scheme("../other.md"));
    assert!(!is_url_scheme("plain.md"));
}

#[test]
fn normalise_collapses_dot_segments() {
    let p = normalise_relative(Path::new("docs/./foo/../bar.md"));
    assert_eq!(p, PathBuf::from("docs/bar.md"));
}

#[test]
fn drop_traversed_strips_descendants() {
    let symlinks = vec![Symlink {
        path: "plugins/spec/skills/merge/references".to_string(),
        target: "../../references".to_string(),
        broken: false,
        resolved_target: Some("plugins/spec/references".to_string()),
    }];
    let mut links = vec![
        MarkdownLink {
            from_path: "plugins/spec/skills/merge/references/artifact-conventions.md".into(),
            to_raw: "../../../docs/x.md".into(),
            line: 1,
            resolves: None,
            image: false,
        },
        MarkdownLink {
            from_path: "plugins/spec/skills/merge/references".into(),
            to_raw: "ignored".into(),
            line: 1,
            resolves: None,
            image: false,
        },
        MarkdownLink {
            from_path: "plugins/spec/references/artifact-conventions.md".into(),
            to_raw: "../../../docs/x.md".into(),
            line: 1,
            resolves: None,
            image: false,
        },
        MarkdownLink {
            from_path: "plugins/spec/skills/merge/references-extra/x.md".into(),
            to_raw: "../sibling.md".into(),
            line: 1,
            resolves: None,
            image: false,
        },
    ];
    drop_symlink_traversed_links(&mut links, &symlinks);
    let kept: Vec<&str> = links.iter().map(|l| l.from_path.as_str()).collect();
    assert_eq!(
        kept,
        vec![
            "plugins/spec/references/artifact-conventions.md",
            "plugins/spec/skills/merge/references-extra/x.md",
        ]
    );
}
