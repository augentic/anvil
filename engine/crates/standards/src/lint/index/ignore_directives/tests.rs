use super::*;

fn file(relative: &str, language: &str, body: &str) -> DiscoveredFile {
    DiscoveredFile {
        relative: relative.into(),
        kind: FileKind::Text,
        language: Some(language.into()),
        bytes: Some(body.as_bytes().to_vec()),
    }
}

fn only(mut directives: Vec<IgnoreDirective>) -> IgnoreDirective {
    assert_eq!(directives.len(), 1, "expected one directive, got {directives:?}");
    directives.pop().expect("checked non-empty")
}

#[test]
fn rust_double_slash_recognised() {
    let f = file(
        "src/lib.rs",
        "rust",
        "// specify-ignore: UNI-014 — Hardcoded for the demo fixture\nlet x = 1;\n",
    );
    let d = only(extract(&f));
    assert_eq!(d.line, 1);
    assert_eq!(d.target_line, 2);
    assert_eq!(d.rule_id, "UNI-014");
    assert_eq!(d.rationale.as_deref(), Some("Hardcoded for the demo fixture"));
    assert!(d.raw.starts_with("// specify-ignore:"));
}

#[test]
fn python_hash_directive_is_recognised() {
    let f = file(
        "scripts/run.py",
        "python",
        "# specify-ignore: UNI-014 -- demo rationale that is long\nresult = compute()\n",
    );
    let d = only(extract(&f));
    assert_eq!(d.line, 1);
    assert_eq!(d.target_line, 2);
    assert_eq!(d.rationale.as_deref(), Some("demo rationale that is long"));
}

#[test]
fn markdown_html_directive_is_recognised() {
    let f = file(
        "docs/note.md",
        "markdown",
        "<!-- specify-ignore: UNI-014 — explained in commit message body -->\n# Heading\n",
    );
    let d = only(extract(&f));
    assert_eq!(d.line, 1);
    assert_eq!(d.target_line, 2);
    assert!(d.raw.ends_with("-->"));
}

#[test]
fn sql_double_dash_directive_is_recognised() {
    let f = file(
        "migrations/001.sql",
        "sql",
        "-- specify-ignore: UNI-014 — legacy schema kept verbatim\nSELECT 1;\n",
    );
    let d = only(extract(&f));
    assert_eq!(d.line, 1);
    assert_eq!(d.target_line, 2);
}

#[test]
fn em_dash_and_hyphen_separators() {
    let em =
        file("a.rs", "rust", "// specify-ignore: UNI-001 — a long enough rationale\nfn f() {}\n");
    let dd =
        file("b.rs", "rust", "// specify-ignore: UNI-001 -- a long enough rationale\nfn f() {}\n");
    assert_eq!(only(extract(&em)).rationale.as_deref(), Some("a long enough rationale"));
    assert_eq!(only(extract(&dd)).rationale.as_deref(), Some("a long enough rationale"));
}

#[test]
fn inline_trailing_targets_own_line() {
    let f = file(
        "src/lib.rs",
        "rust",
        "let x = foo(); // specify-ignore: UNI-014 — inline trailing reason here\n",
    );
    let d = only(extract(&f));
    assert_eq!(d.line, 1);
    assert_eq!(d.target_line, 1);
    assert!(d.raw.starts_with("// specify-ignore:"));
}

#[test]
fn blank_lines_skipped() {
    let f = file(
        "src/lib.rs",
        "rust",
        "// specify-ignore: UNI-014 — blank line skip rationale here\n\n\nlet x = 1;\n",
    );
    let d = only(extract(&f));
    assert_eq!(d.line, 1);
    assert_eq!(d.target_line, 4);
}

#[test]
fn consecutive_compose_same_target() {
    let f = file(
        "src/lib.rs",
        "rust",
        "// specify-ignore: UNI-014 — first rationale that is long\n\
             // specify-ignore: UNI-015 — second rationale that is long\n\
             let x = 1;\n",
    );
    let directives = extract(&f);
    assert_eq!(directives.len(), 2);
    assert_eq!(directives[0].rule_id, "UNI-014");
    assert_eq!(directives[1].rule_id, "UNI-015");
    assert_eq!(directives[0].target_line, 3);
    assert_eq!(directives[1].target_line, 3);
}

#[test]
fn missing_rationale_is_captured_with_none() {
    let f = file("src/lib.rs", "rust", "// specify-ignore: UNI-014\nlet x = 1;\n");
    let d = only(extract(&f));
    assert_eq!(d.rule_id, "UNI-014");
    assert!(d.rationale.is_none());
}

#[test]
fn short_rationale_is_captured_verbatim() {
    let f = file("src/lib.rs", "rust", "// specify-ignore: UNI-014 — short\nlet x = 1;\n");
    let d = only(extract(&f));
    assert_eq!(d.rule_id, "UNI-014");
    assert_eq!(d.rationale.as_deref(), Some("short"));
}

#[test]
fn c_block_comment_recognised() {
    let f = file(
        "src/lib.rs",
        "rust",
        "let x = /* specify-ignore: UNI-014 — block comment rationale */ 1;\n",
    );
    let d = only(extract(&f));
    assert_eq!(d.line, 1);
    assert_eq!(d.target_line, 1);
    assert_eq!(d.rule_id, "UNI-014");
    assert_eq!(d.rationale.as_deref(), Some("block comment rationale"));
    assert!(d.raw.starts_with("/* specify-ignore:"));
    assert!(d.raw.ends_with("*/"));
}

#[test]
fn token_in_string_literal_ignored() {
    let f = file(
        "src/lib.rs",
        "rust",
        "let s = \"specify-ignore: UNI-014 — looks like a directive\";\nlet y = 2;\n",
    );
    assert!(extract(&f).is_empty(), "string-literal token must not match");
}

#[test]
fn target_line_past_eof() {
    let f = file("src/lib.rs", "rust", "// specify-ignore: UNI-014 — terminal directive only\n");
    let d = only(extract(&f));
    assert_eq!(d.line, 1);
    // `split('\n')` yields two segments here ("" trailing) so
    // total_lines == 2 and target_line lands at 3 (past EOF).
    assert_eq!(d.target_line, 3);
}

#[test]
fn placeholder_rule_id_ignored() {
    // The rationale-table example in `docs/reference/ignore-directives.md`
    // (and similar prose) uses the Unicode ellipsis as a stand-in for a
    // real rule id; the extractor must skip such lines so documentation
    // does not self-trigger UNI-022 / UNI-023.
    let f = file(
        "docs/reference/ignore-directives.md",
        "markdown",
        "<!-- specify-ignore: … -->\n# Heading\n",
    );
    assert!(extract(&f).is_empty(), "placeholder rule-id must not parse");
}

#[test]
fn angle_placeholder_rule_id_ignored() {
    let f = file(
        "docs/reference/ignore-directives.md",
        "markdown",
        "<!-- specify-ignore: <RULE-ID> — rationale prose at length -->\n# Heading\n",
    );
    assert!(extract(&f).is_empty(), "<RULE-ID> placeholder must not parse");
}

#[test]
fn languages_outside_list_skipped() {
    let f = file("data.json", "json", "{\"//\": \"specify-ignore: UNI-014 — no comments here\"}\n");
    assert!(extract(&f).is_empty());
}

#[test]
fn binary_files_are_skipped() {
    let f = DiscoveredFile {
        relative: "blob.rs".into(),
        kind: FileKind::Binary,
        language: Some("rust".into()),
        bytes: None,
    };
    assert!(extract(&f).is_empty());
}
