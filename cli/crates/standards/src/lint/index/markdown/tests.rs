use super::*;
use crate::lint::FileKind;

fn markdown(relative: &str, body: &str) -> DiscoveredFile {
    DiscoveredFile {
        relative: relative.into(),
        kind: FileKind::Text,
        language: Some("markdown".into()),
        bytes: Some(body.as_bytes().to_vec()),
    }
}

#[test]
fn sections_capture_atx_headings() {
    let f = markdown("doc.md", "# Top\nintro line\n\n## Sub\nbody one\nbody two\n## Sibling\n");
    let sections = extract_sections(&f);
    assert_eq!(sections.len(), 3);
    assert_eq!(sections[0].title, "Top");
    assert_eq!(sections[0].level, 1);
    assert_eq!(sections[0].line_start, 1);
    // "Top" closes when the next h1-or-shallower heading appears —
    // there is no other h1, so it absorbs everything through EOF.
    assert_eq!(sections[1].title, "Sub");
    assert_eq!(sections[1].line_start, 4);
    assert_eq!(sections[1].line_end, 6);
    assert_eq!(sections[1].body_line_count, 2);
}

#[test]
fn sections_skip_fenced_headings() {
    let f = markdown("doc.md", "# Real\n```rust\n# not a heading\n```\n## Real Sub\n");
    let sections = extract_sections(&f);
    let titles: Vec<&str> = sections.iter().map(|s| s.title.as_str()).collect();
    assert_eq!(titles, vec!["Real", "Real Sub"]);
}

#[test]
fn sections_skip_html_comments() {
    let f = markdown("doc.md", "# Visible\n<!--\n# hidden\n-->\n## Sub\n");
    let sections = extract_sections(&f);
    let titles: Vec<&str> = sections.iter().map(|s| s.title.as_str()).collect();
    assert_eq!(titles, vec!["Visible", "Sub"]);
}

#[test]
fn links_record_relative_targets() {
    let f = markdown("doc.md", "intro [first](./a.md) and [second](https://example.com)\n");
    let links = extract_links(&f);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].to_raw, "./a.md");
    assert_eq!(links[0].line, 1);
    assert_eq!(links[1].to_raw, "https://example.com");
}

#[test]
fn links_skip_fences_and_comments() {
    let f = markdown(
        "doc.md",
        "real [one](./a.md)\n```\n[fake](nope)\n```\n<!-- [also-fake](nope) -->\n",
    );
    let links = extract_links(&f);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].to_raw, "./a.md");
}

#[test]
fn image_links_are_flagged() {
    let f = markdown("doc.md", "logo: ![alt](./logo.png) and [real](./a.md)\n");
    let links = extract_links(&f);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].to_raw, "./logo.png");
    assert!(links[0].image, "image embeds carry the image flag");
    assert_eq!(links[1].to_raw, "./a.md");
    assert!(!links[1].image, "plain links are not flagged as images");
}

#[test]
fn inline_code_links_are_ignored() {
    let f = markdown(
        "doc.md",
        "see `[label](target)` and a real [one](./a.md), plus ``backtick `inside` `` text\n",
    );
    let links = extract_links(&f);
    assert_eq!(links.len(), 1, "only the un-coded link should surface: {links:?}");
    assert_eq!(links[0].to_raw, "./a.md");
}
