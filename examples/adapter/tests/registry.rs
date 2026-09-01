//! The embedded prose registry: the extract prompt and greeting
//! reference ride inside; survey prose does not.

use emery_adapter::SourceAdapter as _;
use emery_adapter::registry::{body, find};
use source::Adapter;

#[test]
fn embeds_extract_prompt() {
    assert!(body(Adapter::docs(), "prompts/extract.md").starts_with("# source.extract"));
}

#[test]
fn embeds_greeting_reference() {
    let doc =
        find(Adapter::docs(), "references/greeting.md").expect("greeting reference is embedded");
    assert!(doc.body.contains("greeting.behaviour"), "reference pins the requirement id");
}

#[test]
fn no_survey_prose() {
    assert!(find(Adapter::docs(), "prompts/survey.md").is_none(), "survey prose is deleted");
}

// No embedded document may exceed the 800 non-blank-line hard cap.
#[test]
fn prose_caps() {
    for doc in Adapter::docs() {
        let lines = doc.body.lines().filter(|line| !line.trim().is_empty()).count();
        assert!(lines <= 800, "{} carries {lines} non-blank lines (cap 800)", doc.path);
    }
}
