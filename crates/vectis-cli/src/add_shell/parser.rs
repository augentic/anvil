//! Structural parser for `shared/src/app.rs`.
//!
//! Chunk 10 exposes a *limited, structural* parse of a scaffolded Crux
//! `app.rs`: we use `syn` to walk the AST looking for the two bits
//! `vectis add-shell` needs, and nothing more:
//!
//! 1. **App struct name** -- the `Foo` in `impl App for Foo { ... }`.
//!    This is what the scaffold uses as `__APP_STRUCT__` / the shell
//!    project name, so getting it right is load-bearing.
//! 2. **Capability set** -- each `type <Alias> = <crux_crate>::<...><...>;`
//!    line that names a Crux capability crate the engine knows about
//!    contributes the corresponding [`Capability`]. Crux-like aliases
//!    whose crate is *not* in our canonical set (e.g. a writer-skill
//!    experiment) land in `unrecognized_capabilities` as warnings, not
//!    errors: the RFC explicitly prefers "scaffold the shell with the
//!    recognized capabilities and surface the rest" over "fail because
//!    we don't recognize a type alias".
//!
//! We intentionally do not try to reconstruct the `Effect` enum or any
//! other Crux construct: capability detection off `type` aliases is
//! sufficient for `add-shell` (which only needs to know which
//! CAP-marker regions to keep), and the Effect enum sometimes skips
//! variants that the aliases name (e.g. the `Sse` cap has no `Effect`
//! variant in the render-only baseline; see chunk-6 notes).

use crate::error::VectisError;
use crate::templates::Capability;

/// Outcome of parsing a scaffolded `shared/src/app.rs`.
#[derive(Debug, Clone)]
pub struct ParsedApp {
    /// App struct name, as it appeared in `impl App for <Name>`.
    pub app_name: String,
    /// Recognized capabilities, in the order they first appeared in the
    /// source. Duplicates (which the scaffold should never produce) are
    /// collapsed -- we treat the source as a set, not a multiset.
    pub capabilities: Vec<Capability>,
    /// `type Foo = crux_*::...;` aliases whose crate is not one of the
    /// canonical Crux capability crates. Reported verbatim (the full
    /// crate path that appeared on the RHS) so the user can see exactly
    /// what tripped the warning.
    pub unrecognized_capabilities: Vec<String>,
}

/// Parse a `shared/src/app.rs` source string.
///
/// Errors:
/// - Returns `InvalidProject` if the source isn't valid Rust (syn
///   failure) -- the message includes the parser's own location info.
/// - Returns `InvalidProject` if no `impl App for <Name>` is found --
///   this is the marker the scaffold writes and its absence means the
///   file is not a vectis-scaffolded Crux app.
pub fn parse_app_rs(source: &str) -> Result<ParsedApp, VectisError> {
    let file = syn::parse_file(source).map_err(|e| VectisError::InvalidProject {
        message: format!(
            "could not parse shared/src/app.rs as Rust: {e} \
             (is this a vectis-scaffolded Crux project?)"
        ),
    })?;

    let app_name = find_app_impl(&file).ok_or_else(|| VectisError::InvalidProject {
        message: "no `impl App for <Name>` found in shared/src/app.rs \
                  (is this a vectis-scaffolded Crux project? -- try `vectis init` instead)"
            .to_string(),
    })?;

    let (capabilities, unrecognized_capabilities) = collect_capabilities(&file);

    Ok(ParsedApp {
        app_name,
        capabilities,
        unrecognized_capabilities,
    })
}

/// Walk items looking for `impl <Trait> for <Type>` where the last
/// segment of `Trait` is `App` and the last segment of `Type` is a bare
/// identifier (e.g. `Counter`, not `Counter<T>` or `some::qualified::Path`).
///
/// Returns the identifier text of the self-type's final segment, which
/// is what the scaffold uses as the app name.
fn find_app_impl(file: &syn::File) -> Option<String> {
    for item in &file.items {
        let syn::Item::Impl(item_impl) = item else {
            continue;
        };
        let Some((_, trait_path, _)) = &item_impl.trait_ else {
            continue;
        };
        let Some(last) = trait_path.segments.last() else {
            continue;
        };
        if last.ident != "App" {
            continue;
        }

        let syn::Type::Path(self_ty) = item_impl.self_ty.as_ref() else {
            continue;
        };
        let Some(last_seg) = self_ty.path.segments.last() else {
            continue;
        };
        // The scaffold always writes a bare `__APP_STRUCT__` with no
        // generics; anything more exotic is a hand-edited app and we
        // prefer to bail out via the "no App impl found" branch above
        // than guess.
        if !matches!(last_seg.arguments, syn::PathArguments::None) {
            continue;
        }
        return Some(last_seg.ident.to_string());
    }
    None
}

/// Walk top-level `type` aliases and classify each by the crate root of
/// its RHS path.
fn collect_capabilities(file: &syn::File) -> (Vec<Capability>, Vec<String>) {
    let mut caps: Vec<Capability> = Vec::new();
    let mut unrecognized: Vec<String> = Vec::new();

    for item in &file.items {
        let syn::Item::Type(ty) = item else {
            continue;
        };
        let syn::Type::Path(type_path) = ty.ty.as_ref() else {
            continue;
        };
        let Some((cap, crate_label)) = classify_cap_path(&type_path.path) else {
            continue;
        };
        match cap {
            Some(c) => {
                if !caps.contains(&c) {
                    caps.push(c);
                }
            }
            None => {
                if !unrecognized.contains(&crate_label) {
                    unrecognized.push(crate_label);
                }
            }
        }
    }

    (caps, unrecognized)
}

/// Classify the RHS of a `type <alias> = <path>;` aliasing statement.
///
/// Returns `Some((Some(Cap), label))` for a recognized capability,
/// `Some((None, label))` for a `crux_*` alias whose crate we don't
/// recognize, and `None` for any other alias (e.g. `type Foo = Vec<u8>;`
/// -- not a capability at all, skip silently).
///
/// The returned `label` is the display form the RFC asks us to emit in
/// `unrecognized_capabilities` -- the crate segment of the RHS path (or
/// `crux_http::sse` for the nested Sse case), not the full path with
/// generics.
fn classify_cap_path(path: &syn::Path) -> Option<(Option<Capability>, String)> {
    let first = path.segments.first()?;
    let first_ident = first.ident.to_string();

    // `type Sse = crux_http::sse::Sse<...>;` -- the special nested case
    // the chunk-10 spec enumerates. Detect it by the second segment
    // being `sse` rather than by the alias name.
    if first_ident == "crux_http" {
        if path.segments.len() >= 2 && path.segments[1].ident == "sse" {
            return Some((Some(Capability::Sse), "crux_http::sse".to_string()));
        }
        return Some((Some(Capability::Http), "crux_http".to_string()));
    }

    let cap = match first_ident.as_str() {
        "crux_kv" => Some(Capability::Kv),
        "crux_time" => Some(Capability::Time),
        "crux_platform" => Some(Capability::Platform),
        other if other.starts_with("crux_") => {
            return Some((None, other.to_string()));
        }
        _ => return None,
    };
    Some((cap, first_ident))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_source(body: &str) -> String {
        // Minimal valid Rust scaffold with the struct + App impl so the
        // parser doesn't short-circuit on "no App impl".
        format!(
            r#"use crux_core::App;

{body}

#[derive(Default)]
pub struct Counter;

impl App for Counter {{
    type Event = ();
    type Model = ();
    type ViewModel = ();
    type Effect = ();
    fn update(&self, _event: (), _model: &mut ()) -> () {{}}
    fn view(&self, _model: &()) -> () {{}}
}}
"#
        )
    }

    #[test]
    fn extracts_app_name_from_impl() {
        let parsed = parse_app_rs(&sample_source("")).unwrap();
        assert_eq!(parsed.app_name, "Counter");
        assert!(parsed.capabilities.is_empty());
        assert!(parsed.unrecognized_capabilities.is_empty());
    }

    #[test]
    fn extracts_http_capability() {
        let src = sample_source("type Http = crux_http::Http<Effect, Event>;");
        let parsed = parse_app_rs(&src).unwrap();
        assert_eq!(parsed.capabilities, vec![Capability::Http]);
    }

    #[test]
    fn extracts_kv_capability_from_keyvalue_alias() {
        // The scaffold uses `type KeyValue = crux_kv::KeyValue<...>;`
        // (not `type Kv = ...`). The parser keys off the RHS crate, not
        // the alias name, so this must still classify as `Kv`.
        let src = sample_source("type KeyValue = crux_kv::KeyValue<Effect, Event>;");
        let parsed = parse_app_rs(&src).unwrap();
        assert_eq!(parsed.capabilities, vec![Capability::Kv]);
    }

    #[test]
    fn extracts_time_capability() {
        let src = sample_source("type Time = crux_time::Time<Effect, Event>;");
        let parsed = parse_app_rs(&src).unwrap();
        assert_eq!(parsed.capabilities, vec![Capability::Time]);
    }

    #[test]
    fn extracts_platform_capability() {
        let src = sample_source("type Platform = crux_platform::Platform<Effect, Event>;");
        let parsed = parse_app_rs(&src).unwrap();
        assert_eq!(parsed.capabilities, vec![Capability::Platform]);
    }

    #[test]
    fn extracts_sse_capability_from_nested_path() {
        // Sse lives under `crux_http::sse`, and the classifier must see
        // the `sse` segment *before* the `crux_http` rule returns Http.
        let src = sample_source("type Sse = crux_http::sse::Sse<Effect, Event>;");
        let parsed = parse_app_rs(&src).unwrap();
        assert_eq!(parsed.capabilities, vec![Capability::Sse]);
    }

    #[test]
    fn extracts_full_capability_matrix_in_source_order() {
        let src = sample_source(
            "type Http = crux_http::Http<Effect, Event>;\n\
             type KeyValue = crux_kv::KeyValue<Effect, Event>;\n\
             type Time = crux_time::Time<Effect, Event>;\n\
             type Platform = crux_platform::Platform<Effect, Event>;",
        );
        let parsed = parse_app_rs(&src).unwrap();
        assert_eq!(
            parsed.capabilities,
            vec![
                Capability::Http,
                Capability::Kv,
                Capability::Time,
                Capability::Platform,
            ]
        );
    }

    #[test]
    fn dedupes_duplicate_capabilities_without_erroring() {
        let src = sample_source(
            "type Http = crux_http::Http<Effect, Event>;\n\
             type AnotherHttp = crux_http::Http<Effect, Event>;",
        );
        let parsed = parse_app_rs(&src).unwrap();
        assert_eq!(parsed.capabilities, vec![Capability::Http]);
    }

    #[test]
    fn classifies_unknown_crux_crates_as_unrecognized() {
        let src = sample_source("type Exp = crux_experimental::Thing<Effect, Event>;");
        let parsed = parse_app_rs(&src).unwrap();
        assert!(parsed.capabilities.is_empty());
        assert_eq!(parsed.unrecognized_capabilities, vec!["crux_experimental"]);
    }

    #[test]
    fn ignores_non_crux_type_aliases() {
        // `type Foo = Vec<u8>;` is a perfectly valid type alias but
        // nothing to do with capabilities -- must not appear anywhere.
        let src = sample_source("type Foo = Vec<u8>;");
        let parsed = parse_app_rs(&src).unwrap();
        assert!(parsed.capabilities.is_empty());
        assert!(parsed.unrecognized_capabilities.is_empty());
    }

    #[test]
    fn rejects_source_with_no_app_impl() {
        let src = "pub struct NotAnApp;\n";
        let err = parse_app_rs(src).expect_err("missing App impl must fail");
        match err {
            VectisError::InvalidProject { message } => {
                assert!(message.contains("no `impl App for <Name>` found"));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn rejects_source_that_is_not_valid_rust() {
        let err = parse_app_rs("this is not rust {{{").expect_err("invalid syntax must fail");
        match err {
            VectisError::InvalidProject { message } => {
                assert!(message.contains("could not parse"), "{message}");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_a_fully_scaffolded_app_rs_template() {
        // Sanity-check against the actual template text: the chunk-5
        // scaffold emits an `app.rs` with every CAP region present (so
        // what the parser sees here is the "all caps on" shape after
        // the engine drops the marker lines).
        let src = r#"use crux_core::App;
use crux_http::HttpRequest;
use crux_kv::KeyValueOperation;
use crux_time::TimeRequest;
use crux_platform::PlatformRequest;

type Http = crux_http::Http<Effect, Event>;
type KeyValue = crux_kv::KeyValue<Effect, Event>;
type Time = crux_time::Time<Effect, Event>;
type Platform = crux_platform::Platform<Effect, Event>;

pub struct Counter;

impl App for Counter {
    type Event = ();
    type Model = ();
    type ViewModel = ();
    type Effect = ();
    fn update(&self, _event: (), _model: &mut ()) -> () {}
    fn view(&self, _model: &()) -> () {}
}
"#;
        let parsed = parse_app_rs(src).unwrap();
        assert_eq!(parsed.app_name, "Counter");
        assert_eq!(
            parsed.capabilities,
            vec![
                Capability::Http,
                Capability::Kv,
                Capability::Time,
                Capability::Platform,
            ]
        );
    }
}
