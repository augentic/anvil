//! Mirror-parity guard: one argv → `Input` extraction per routed
//! command.
//!
//! Every routing arm in the shims is `cli::post::<Handler, _, _>(…,
//! args)` / `cli::get` over a mirror `*Args` struct; the bridge's one
//! serde round-trip ([`cli::front::extract`]) is the only conversion.
//! These tests parse a sample argv line through the shared grammar,
//! run exactly that extraction, and assert the resulting handler
//! `Input` — so a mirror field drifting from its `Input` (a rename, a
//! type change, a lost default) fails here instead of at run time on
//! one transport.
//!
//! Handler behaviour stays out: `crates/workflow/tests` drives the
//! `Handler` layer transport-free.

use cli::cli::{Commands, parse};
use cli::commands::archive::cli::ArchiveAction;
use cli::commands::journal::cli::JournalAction;
use cli::commands::plan::cli::PlanAction;
use cli::commands::registry::cli::RegistryAction;
use cli::commands::slice::cli::{SliceAction, SliceMergeAction, SliceModelAction, SliceTaskAction};
use cli::commands::source::cli::SourceAction;
use cli::commands::target::cli::TargetAction;

/// Parse one argv line (program name included), match the expected
/// action variant (the pattern binds the mirror as `args`), and run
/// the bridge extraction into the handler `Input` type.
macro_rules! extract {
    ($input:ty, $pat:pat => $args:ident, [$($arg:expr),+ $(,)?]) => {{
        let argv = [$($arg),+];
        let cli = parse(argv.iter().map(ToString::to_string)).unwrap_or_else(|exit| {
            panic!("argv {argv:?} failed to parse (exit {})", exit.code());
        });
        let $pat = cli.command else {
            panic!("argv {argv:?} parsed to an unexpected variant");
        };
        let input: $input = cli::front::extract($args).expect("mirror extracts onto Input");
        input
    }};
}

mod source {
    use workflow::adapter::handlers::ResolveInput;
    use workflow::orchestrate::handlers::{ExtractInput, SurveyInput};

    use super::*;

    #[test]
    fn resolve() {
        let input = extract!(
            ResolveInput,
            Commands::Source {
                action: SourceAction::Resolve(args)
            } => args,
            ["specify", "source", "resolve", "typescript"]
        );
        assert_eq!(input.value, "typescript");
        // The `--project-dir` default rides the mirror, not the Input.
        assert_eq!(input.project_dir.as_deref(), Some(std::path::Path::new(".")));
    }

    #[test]
    fn survey() {
        let input = extract!(
            SurveyInput,
            Commands::Source {
                action: SourceAction::Survey(args)
            } => args,
            ["specify", "source", "survey", "docs", "--plan", "account-revamp"]
        );
        assert_eq!(input.source, "docs");
        assert_eq!(input.plan.as_deref(), Some("account-revamp"));
    }

    #[test]
    fn extract() {
        let input = extract!(
            ExtractInput,
            Commands::Source {
                action: SourceAction::Extract(args)
            } => args,
            ["specify", "source", "extract", "docs", "billing-lead", "--slice", "billing"]
        );
        assert_eq!(input.source, "docs");
        assert_eq!(input.lead, "billing-lead");
        assert_eq!(input.slice, "billing");
    }
}

mod target {
    use workflow::adapter::handlers::ResolveInput;

    use super::*;

    #[test]
    fn resolve() {
        let input = extract!(
            ResolveInput,
            Commands::Target {
                action: TargetAction::Resolve(args)
            } => args,
            ["specify", "target", "resolve", "omnia@1.0.0"]
        );
        assert_eq!(input.value, "omnia@1.0.0");
        assert_eq!(input.project_dir.as_deref(), Some(std::path::Path::new(".")));
    }
}

mod slice {
    use workflow::orchestrate::handlers::{BuildInput, MergeRunInput, RefineInput};
    use workflow::slice::handlers::{
        ConflictCheckInput, CreateInput, DropInput, ModelShowInput, OverlapInput, PreviewInput,
        ProvenanceInput, TaskMarkInput, TaskProgressInput, TouchedSpecsInput, TransitionInput,
        ValidateInput,
    };
    use workflow::slice::{CreateIfExists, LifecycleStatus};

    use super::*;

    #[test]
    fn create() {
        let input = extract!(
            CreateInput,
            Commands::Slice {
                action: SliceAction::Create(args)
            } => args,
            ["specify", "slice", "create", "billing", "--target", "omnia", "--if-exists", "restart"]
        );
        assert_eq!(input.name, "billing");
        assert_eq!(input.target.as_deref(), Some("omnia"));
        assert_eq!(input.if_exists, CreateIfExists::Restart);
    }

    #[test]
    fn create_if_exists_defaults_to_fail() {
        let input = extract!(
            CreateInput,
            Commands::Slice {
                action: SliceAction::Create(args)
            } => args,
            ["specify", "slice", "create", "billing"]
        );
        assert_eq!(input.if_exists, CreateIfExists::Fail);
    }

    #[test]
    fn validate() {
        let input = extract!(
            ValidateInput,
            Commands::Slice {
                action: SliceAction::Validate(args)
            } => args,
            ["specify", "slice", "validate", "billing"]
        );
        assert_eq!(input.name, "billing");
    }

    #[test]
    fn provenance() {
        let input = extract!(
            ProvenanceInput,
            Commands::Slice {
                action: SliceAction::Provenance(args)
            } => args,
            ["specify", "slice", "provenance", "billing"]
        );
        assert_eq!(input.name, "billing");
    }

    #[test]
    fn model_show() {
        let input = extract!(
            ModelShowInput,
            Commands::Slice {
                action: SliceAction::Model {
                    action: SliceModelAction::Show(args)
                }
            } => args,
            ["specify", "slice", "model", "show", "billing"]
        );
        assert_eq!(input.name, "billing");
    }

    #[test]
    fn refine() {
        let input = extract!(
            RefineInput,
            Commands::Slice {
                action: SliceAction::Refine(args)
            } => args,
            ["specify", "slice", "refine", "billing"]
        );
        assert_eq!(input.name, "billing");
    }

    #[test]
    fn build() {
        let input = extract!(
            BuildInput,
            Commands::Slice {
                action: SliceAction::Build(args)
            } => args,
            ["specify", "slice", "build", "billing"]
        );
        assert_eq!(input.name, "billing");
    }

    #[test]
    fn merge_run() {
        let input = extract!(
            MergeRunInput,
            Commands::Slice {
                action: SliceAction::Merge {
                    action: SliceMergeAction::Run(args)
                }
            } => args,
            ["specify", "slice", "merge", "run", "billing", "--allow-composition-replace"]
        );
        assert_eq!(input.name, "billing");
        assert!(input.allow_composition_replace);
    }

    #[test]
    fn merge_preview() {
        let input = extract!(
            PreviewInput,
            Commands::Slice {
                action: SliceAction::Merge {
                    action: SliceMergeAction::Preview(args)
                }
            } => args,
            ["specify", "slice", "merge", "preview", "billing"]
        );
        assert_eq!(input.name, "billing");
    }

    #[test]
    fn merge_conflict_check() {
        let input = extract!(
            ConflictCheckInput,
            Commands::Slice {
                action: SliceAction::Merge {
                    action: SliceMergeAction::ConflictCheck(args)
                }
            } => args,
            ["specify", "slice", "merge", "conflict-check", "billing"]
        );
        assert_eq!(input.name, "billing");
    }

    #[test]
    fn task_progress() {
        let input = extract!(
            TaskProgressInput,
            Commands::Slice {
                action: SliceAction::Task {
                    action: SliceTaskAction::Progress(args)
                }
            } => args,
            ["specify", "slice", "task", "progress", "billing"]
        );
        assert_eq!(input.name, "billing");
    }

    #[test]
    fn task_mark() {
        let input = extract!(
            TaskMarkInput,
            Commands::Slice {
                action: SliceAction::Task {
                    action: SliceTaskAction::Mark(args)
                }
            } => args,
            ["specify", "slice", "task", "mark", "billing", "1.1"]
        );
        assert_eq!(input.name, "billing");
        assert_eq!(input.task_number, "1.1");
    }

    #[test]
    fn transition() {
        let input = extract!(
            TransitionInput,
            Commands::Slice {
                action: SliceAction::Transition(args)
            } => args,
            ["specify", "slice", "transition", "billing", "refined"]
        );
        assert_eq!(input.name, "billing");
        assert_eq!(input.target, LifecycleStatus::Refined);
    }

    #[test]
    fn touched_specs() {
        let input = extract!(
            TouchedSpecsInput,
            Commands::Slice {
                action: SliceAction::TouchedSpecs(args)
            } => args,
            ["specify", "slice", "touched-specs", "billing", "--set", "omnia:new,vectis:modified"]
        );
        assert_eq!(input.name, "billing");
        assert!(!input.scan);
        assert_eq!(input.set, ["omnia:new", "vectis:modified"]);
    }

    #[test]
    fn overlap() {
        let input = extract!(
            OverlapInput,
            Commands::Slice {
                action: SliceAction::Overlap(args)
            } => args,
            ["specify", "slice", "overlap", "billing"]
        );
        assert_eq!(input.name, "billing");
    }

    #[test]
    fn drop() {
        let input = extract!(
            DropInput,
            Commands::Slice {
                action: SliceAction::Drop(args)
            } => args,
            ["specify", "slice", "drop", "billing", "--reason", "superseded"]
        );
        assert_eq!(input.name, "billing");
        assert_eq!(input.reason.as_deref(), Some("superseded"));
    }
}

mod plan {
    use artifacts::evidence::ClaimKind;
    use workflow::change::plan::handlers::{
        AddInput, AmendInput, ArchiveInput, CreateInput, NextInput, RemoveInput, StatusInput,
        TransitionInput, ValidateInput,
    };
    use workflow::orchestrate::handlers::{AuthorInput, ExecuteInput};

    use super::*;

    #[test]
    fn create() {
        let input = extract!(
            CreateInput,
            Commands::Plan {
                action: PlanAction::Create(args)
            } => args,
            [
                "specify",
                "plan",
                "create",
                "account-revamp",
                "--source",
                "docs=documentation:./notes",
                "--intent",
                "revamp the account area",
                "--auto-approve",
                "--authority-override",
                "billing",
                "decision=docs",
            ]
        );
        assert_eq!(input.name, "account-revamp");
        assert_eq!(input.sources.len(), 1);
        assert_eq!(input.sources[0].key, "docs");
        assert_eq!(input.sources[0].adapter, "documentation");
        assert_eq!(input.sources[0].path.as_deref(), Some("./notes"));
        assert_eq!(input.intent.as_deref(), Some("revamp the account area"));
        assert!(input.auto_approve);
        assert_eq!(input.authority_override, ["billing", "decision=docs"]);
    }

    #[test]
    fn create_value_bound_source() {
        let input = extract!(
            CreateInput,
            Commands::Plan {
                action: PlanAction::Create(args)
            } => args,
            ["specify", "plan", "create", "demo", "--source", "intent=intent:value:fix: greetings"]
        );
        // The `value:` sentinel switches to literal mode; the literal
        // keeps its own colons.
        assert_eq!(input.sources[0].key, "intent");
        assert_eq!(input.sources[0].adapter, "intent");
        assert_eq!(input.sources[0].path, None);
        assert_eq!(input.sources[0].value.as_deref(), Some("fix: greetings"));
    }

    #[test]
    fn validate() {
        extract!(
            ValidateInput,
            Commands::Plan {
                action: PlanAction::Validate(args)
            } => args,
            ["specify", "plan", "validate"]
        );
    }

    #[test]
    fn next() {
        extract!(
            NextInput,
            Commands::Plan {
                action: PlanAction::Next(args)
            } => args,
            ["specify", "plan", "next"]
        );
    }

    #[test]
    fn status() {
        extract!(
            StatusInput,
            Commands::Plan {
                action: PlanAction::Status(args)
            } => args,
            ["specify", "plan", "status"]
        );
    }

    #[test]
    fn add() {
        let input = extract!(
            AddInput,
            Commands::Plan {
                action: PlanAction::Add(args)
            } => args,
            [
                "specify",
                "plan",
                "add",
                "billing",
                "--depends-on",
                "auth",
                "--sources",
                "docs=billing-lead",
                "--sources",
                "runtime",
                "--description",
                "billing extraction",
                "--project",
                "storefront",
                "--context",
                "specs/billing.md",
                "--authority-override",
                "decision=docs",
            ]
        );
        assert_eq!(input.name, "billing");
        assert_eq!(input.depends_on, ["auth"]);
        assert_eq!(input.sources.len(), 2);
        assert_eq!(input.sources[0].key, "docs");
        assert_eq!(input.sources[0].lead.as_deref(), Some("billing-lead"));
        assert_eq!(input.sources[1].key, "runtime");
        assert_eq!(input.sources[1].lead, None, "bare key is the shorthand");
        assert_eq!(input.description.as_deref(), Some("billing extraction"));
        assert_eq!(input.project.as_deref(), Some("storefront"));
        assert_eq!(input.context, ["specs/billing.md"]);
        assert_eq!(input.authority_override.len(), 1);
        assert!(matches!(input.authority_override[0].kind, ClaimKind::Decision));
        assert_eq!(input.authority_override[0].source, "docs");
    }

    #[test]
    fn amend() {
        let input = extract!(
            AmendInput,
            Commands::Plan {
                action: PlanAction::Amend(args)
            } => args,
            [
                "specify",
                "plan",
                "amend",
                "billing",
                "--sources",
                "docs=billing-lead",
                "--add-source",
                "runtime",
                "--remove-source",
                "captures",
                "--divergence",
                "likely",
                "--authority-override",
                "billing",
                "decision=docs",
                "--clear-authority-override",
                "billing",
                "example",
                "--clear-authority-overrides",
                "checkout",
            ]
        );
        assert_eq!(input.name, "billing");
        assert_eq!(input.depends_on, None, "omitted flag leaves the field unchanged");
        let sources = input.sources.expect("--sources replaces wholesale");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].key, "docs");
        assert_eq!(sources[0].lead.as_deref(), Some("billing-lead"));
        assert_eq!(input.add_source.len(), 1);
        assert_eq!(input.add_source[0].key, "runtime");
        assert_eq!(input.remove_source, ["captures"]);
        assert_eq!(input.divergence.as_deref(), Some("likely"));
        assert_eq!(input.authority_override, ["billing", "decision=docs"]);
        assert_eq!(input.clear_authority_override, ["billing", "example"]);
        assert_eq!(input.clear_authority_overrides, ["checkout"]);
    }

    #[test]
    fn remove() {
        let input = extract!(
            RemoveInput,
            Commands::Plan {
                action: PlanAction::Remove(args)
            } => args,
            ["specify", "plan", "remove", "billing"]
        );
        assert_eq!(input.name, "billing");
    }

    #[test]
    fn transition() {
        let input = extract!(
            TransitionInput,
            Commands::Plan {
                action: PlanAction::Transition(args)
            } => args,
            ["specify", "plan", "transition", "account-revamp", "approved", "--actor", "agent"]
        );
        assert_eq!(input.name, "account-revamp");
        assert_eq!(input.target.as_deref(), Some("approved"));
        assert!(!input.undo);
        assert_eq!(input.actor, "agent");
    }

    #[test]
    fn transition_undo() {
        let input = extract!(
            TransitionInput,
            Commands::Plan {
                action: PlanAction::Transition(args)
            } => args,
            ["specify", "plan", "transition", "billing", "--undo"]
        );
        assert_eq!(input.target, None);
        assert!(input.undo);
        assert_eq!(input.actor, "operator", "the mirror default rides the wire");
    }

    #[test]
    fn author() {
        let input = extract!(
            AuthorInput,
            Commands::Plan {
                action: PlanAction::Author(args)
            } => args,
            [
                "specify",
                "plan",
                "author",
                "account-revamp",
                "--source",
                "docs=documentation:./notes",
                "--intent",
                "revamp the account area",
            ]
        );
        assert_eq!(input.name, "account-revamp");
        assert_eq!(input.sources.len(), 1);
        assert_eq!(input.sources[0].adapter, "documentation");
        assert_eq!(input.intent.as_deref(), Some("revamp the account area"));
    }

    #[test]
    fn execute() {
        extract!(
            ExecuteInput,
            Commands::Plan {
                action: PlanAction::Execute(args)
            } => args,
            ["specify", "plan", "execute"]
        );
    }

    #[test]
    fn archive() {
        let input = extract!(
            ArchiveInput,
            Commands::Plan {
                action: PlanAction::Archive(args)
            } => args,
            ["specify", "plan", "archive", "--force"]
        );
        assert!(input.force);
    }
}

mod journal {
    use workflow::journal::handlers::{EmitInput, ShowInput};

    use super::*;

    #[test]
    fn emit() {
        let input = extract!(
            EmitInput,
            Commands::Journal {
                action: JournalAction::Emit(args)
            } => args,
            [
                "specify",
                "journal",
                "emit",
                "slice.build.started",
                "--payload",
                r#"{"slice-name":"billing"}"#,
            ]
        );
        assert_eq!(input.event, "slice.build.started");
        assert_eq!(input.payload.as_deref(), Some(r#"{"slice-name":"billing"}"#));
    }

    #[test]
    fn show() {
        // GET-side type coercion: `--limit 5` lands as `usize`.
        let input = extract!(
            ShowInput,
            Commands::Journal {
                action: JournalAction::Show(args)
            } => args,
            ["specify", "journal", "show", "--filter", "slice.build", "--limit", "5"]
        );
        assert_eq!(input.filter.as_deref(), Some("slice.build"));
        assert_eq!(input.limit, Some(5));
    }
}

mod registry {
    use workflow::registry::handlers::{AddInput, RemoveInput, ValidateInput};

    use super::*;

    #[test]
    fn validate() {
        extract!(
            ValidateInput,
            Commands::Registry {
                action: RegistryAction::Validate(args)
            } => args,
            ["specify", "registry", "validate"]
        );
    }

    #[test]
    fn add() {
        let input = extract!(
            AddInput,
            Commands::Registry {
                action: RegistryAction::Add(args)
            } => args,
            [
                "specify",
                "registry",
                "add",
                "storefront",
                "--url",
                "git@github.com:acme/storefront.git",
                "--adapter",
                "omnia",
                "--description",
                "the storefront service",
            ]
        );
        assert_eq!(input.name, "storefront");
        assert_eq!(input.url, "git@github.com:acme/storefront.git");
        assert_eq!(input.adapter.as_deref(), Some("omnia"));
        assert_eq!(input.description.as_deref(), Some("the storefront service"));
    }

    #[test]
    fn remove() {
        let input = extract!(
            RemoveInput,
            Commands::Registry {
                action: RegistryAction::Remove(args)
            } => args,
            ["specify", "registry", "remove", "storefront"]
        );
        assert_eq!(input.name, "storefront");
    }
}

mod archive {
    use workflow::slice::handlers::PruneInput;

    use super::*;

    #[test]
    fn prune() {
        let input = extract!(
            PruneInput,
            Commands::Archive {
                action: ArchiveAction::Prune(args)
            } => args,
            ["specify", "archive", "prune", "--keep", "10", "--older-than", "30", "--dry-run"]
        );
        assert_eq!(input.keep, Some(10));
        assert_eq!(input.older_than, Some(30));
        assert!(input.dry_run);
    }
}

mod init {
    use workflow::init::handlers::ScaffoldInput;

    use super::*;

    #[test]
    fn scaffold() {
        // The routing arm forwards `InitArgs` whole; the provisioning
        // flags (`upgrade`, `scaffold-only`) are ignored keys on the
        // wire.
        let input = extract!(
            ScaffoldInput,
            Commands::Init(args) => args,
            [
                "specify",
                "init",
                "omnia",
                "--name",
                "storefront",
                "--description",
                "the storefront service",
                "--platforms",
                "core,ios",
                "--scaffold-only",
            ]
        );
        assert_eq!(input.adapter.as_deref(), Some("omnia"));
        assert_eq!(input.name.as_deref(), Some("storefront"));
        assert_eq!(input.description.as_deref(), Some("the storefront service"));
        assert!(!input.workspace);
        assert_eq!(input.platforms.as_deref(), Some("core,ios"));
    }

    #[test]
    fn scaffold_workspace() {
        let input = extract!(
            ScaffoldInput,
            Commands::Init(args) => args,
            ["specify", "init", "--workspace", "--scaffold-only"]
        );
        assert_eq!(input.adapter, None);
        assert!(input.workspace);
    }
}
