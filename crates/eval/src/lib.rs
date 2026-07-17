//! Adapters linked into the shared native harness.

harness::adapters! {
    pub Adapters {
        source fixture::Fixture,
        source fixture::FixtureDocs,
        source fixture::FixtureCode,
        source fixture::FailSurvey,
        source fixture::FailExtract,
        target fixture::Fixture,
        target fixture::FailGuidance,
        target fixture::FailBuild,
        target fixture::FailMerge,
        target fixture::MissingOutput,
    }
}
