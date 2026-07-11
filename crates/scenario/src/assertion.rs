use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

/// Stable assertion identifiers shared by canonical scenarios and reports.
///
/// This enum is intentionally exhaustive: adding an assertion is a contract
/// change that must also add registry metadata and a schema enum value.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
#[expect(missing_docs, reason = "wire identifiers are documented by the assertion registry")]
pub enum AssertionId {
    PlanExists,
    PlanValidates,
    ExecuteLoopAllDone,
    IntentSingleLead,
    #[serde(rename = "gate-1-not-auto-stamped")]
    #[strum(serialize = "gate-1-not-auto-stamped")]
    Gate1NotAutoStamped,
    SourcesIntentOnly,
    RefineReachesRefined,
    SingleSliceFromDoc,
    SourcesDocumentationOnly,
    MultipleSlicesProposed,
    CrossCuttingLeadMultiHomed,
    ProposeEditRejectLoop,
    #[serde(rename = "gate-1-amendment")]
    #[strum(serialize = "gate-1-amendment")]
    Gate1Amendment,
    MultipleSlicesFromCode,
    SourcesLegacyOnly,
    NoUnderSlicing,
    MergedSliceCombinesSources,
    TentativeMergeSurfaced,
    AmendOverridesMerge,
    ExtractRunsPerContributingSource,
    SlicesMatchExpectedShape,
    NoProjectRoutingRequired,
    ContractSliceFirst,
    ImplementationSlicesRouted,
    DependenciesContractBeforeImplementations,
    DraftStopsAtHandoff,
    ReviewStepNoOp,
    WorkspaceBranchesPrepared,
    PublicationCompleteBeforeFinalize,
    FinalizeArchivesPlan,
    ArchivedPlanPathRecorded,
    ArchivedChangeMdPresent,
    PublicationConfirmationRecorded,
    RerunFinalizePlanNotFound,
    SpecReflectsShapeIdioms,
    DesignReflectsShapeIdioms,
    IntentAndDocFixturesAgree,
    BreakoutStateConsistent,
    ExecuteResumesWithoutFlags,
    BuildFailureStopHint,
    BuildResumesFromFailedTask,
    LoopContinuesToMerge,
    PerSliceProjectRouting,
    SlotsMaterialised,
    PlanLockAtWorkspace,
    BreakoutRoutesToSlot,
    ActiveSliceResolvedAcrossBoundary,
    ChdirWithoutOperatorIntervention,
    GuestLoopDrained,
    GuestJournalCadence,
    GuestGeneratedCrateVerifies,
    GuestMarkerReleased,
    GuestSpecSensible,
    DirtySlotPreserved,
    SliceStatePreserved,
    ResumeContinuesFromInProgress,
    ComposedInitSucceeds,
    ProjectScaffoldWritten,
    ComposedPlanDrained,
    ComposedArtifactsComplete,
    ComposedBaselineMergeVisible,
}

/// Grading mechanism attached to an assertion identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssertionKind {
    /// Deterministic probe with a machine-verifiable verdict.
    Hard,
    /// Evidence-backed qualitative judgment.
    Semantic,
}

/// Registry entry for one stable assertion identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssertionMetadata {
    /// Stable wire identifier.
    pub id: AssertionId,
    /// Required grading mechanism.
    pub kind: AssertionKind,
}

macro_rules! registry {
    ($($kind:ident => [$($id:ident),+ $(,)?]),+ $(,)?) => {
        static ASSERTIONS: &[AssertionMetadata] = &[
            $($(
                AssertionMetadata {
                    id: AssertionId::$id,
                    kind: AssertionKind::$kind,
                },
            )+)+
        ];
    };
}

registry! {
    Hard => [
        PlanExists,
        PlanValidates,
        ExecuteLoopAllDone,
        IntentSingleLead,
        Gate1NotAutoStamped,
        SourcesIntentOnly,
        RefineReachesRefined,
        SingleSliceFromDoc,
        SourcesDocumentationOnly,
        MultipleSlicesProposed,
        CrossCuttingLeadMultiHomed,
        ProposeEditRejectLoop,
        Gate1Amendment,
        MultipleSlicesFromCode,
        SourcesLegacyOnly,
        MergedSliceCombinesSources,
        AmendOverridesMerge,
        ExtractRunsPerContributingSource,
        NoProjectRoutingRequired,
        ContractSliceFirst,
        ImplementationSlicesRouted,
        DependenciesContractBeforeImplementations,
        DraftStopsAtHandoff,
        ReviewStepNoOp,
        WorkspaceBranchesPrepared,
        PublicationCompleteBeforeFinalize,
        FinalizeArchivesPlan,
        ArchivedChangeMdPresent,
        RerunFinalizePlanNotFound,
        BreakoutStateConsistent,
        ExecuteResumesWithoutFlags,
        BuildFailureStopHint,
        BuildResumesFromFailedTask,
        LoopContinuesToMerge,
        PerSliceProjectRouting,
        SlotsMaterialised,
        PlanLockAtWorkspace,
        BreakoutRoutesToSlot,
        ActiveSliceResolvedAcrossBoundary,
        GuestLoopDrained,
        GuestJournalCadence,
        GuestGeneratedCrateVerifies,
        GuestMarkerReleased,
        DirtySlotPreserved,
        SliceStatePreserved,
        ResumeContinuesFromInProgress,
        ComposedInitSucceeds,
        ProjectScaffoldWritten,
        ComposedPlanDrained,
        ComposedArtifactsComplete,
        ComposedBaselineMergeVisible,
    ],
    Semantic => [
        NoUnderSlicing,
        TentativeMergeSurfaced,
        SlicesMatchExpectedShape,
        ArchivedPlanPathRecorded,
        PublicationConfirmationRecorded,
        SpecReflectsShapeIdioms,
        DesignReflectsShapeIdioms,
        IntentAndDocFixturesAgree,
        ChdirWithoutOperatorIntervention,
        GuestSpecSensible,
    ],
}

/// Return the complete assertion metadata registry.
#[must_use]
pub fn assertion_registry() -> &'static [AssertionMetadata] {
    ASSERTIONS
}

impl AssertionId {
    /// Return this identifier's grading metadata.
    ///
    /// # Panics
    ///
    /// Panics only if the compile-time registry omits a closed enum variant.
    #[must_use]
    pub fn metadata(self) -> &'static AssertionMetadata {
        ASSERTIONS
            .iter()
            .find(|metadata| metadata.id == self)
            .expect("every closed AssertionId variant has registry metadata")
    }
}
