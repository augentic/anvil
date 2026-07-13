//! Source-interface export and adapter-to-WIT conversions.

use crate::FixtureAdapter;
use crate::bindings::exports::specify::adapter::source;

impl source::Guest for FixtureAdapter {
    fn metadata(_id: source::AdapterId) -> source::AdapterMetadata {
        source::AdapterMetadata { specify_floor: None }
    }

    async fn survey(id: source::AdapterId) -> Result<Vec<source::Lead>, source::Error> {
        let leads = adapter::survey(&id).map_err(map_error)?;
        Ok(leads.into_iter().map(wire_lead).collect())
    }

    async fn extract(
        id: source::AdapterId, lead: source::Lead,
    ) -> Result<source::Evidence, source::Error> {
        let core_lead = adapter::Lead {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        };
        let evidence = adapter::extract(&id, &core_lead).map_err(map_error)?;
        Ok(source::Evidence {
            authority: wire_authority(evidence.authority),
            claims: evidence.claims.into_iter().map(wire_claim).collect(),
        })
    }
}

fn map_error(error: adapter::Error) -> source::Error {
    match error {
        adapter::Error::InvalidRequest(detail) => source::Error::InvalidRequest(detail),
        adapter::Error::Io(detail) => source::Error::Io(detail),
        adapter::Error::Internal(detail) => source::Error::Internal(detail),
    }
}

fn wire_lead(lead: adapter::Lead) -> source::Lead {
    source::Lead {
        lead: lead.lead,
        synopsis: lead.synopsis,
        topics: lead.topics,
    }
}

const fn wire_authority(authority: adapter::Authority) -> source::Authority {
    match authority {
        adapter::Authority::Intent => source::Authority::Intent,
        adapter::Authority::Documentation => source::Authority::Documentation,
        adapter::Authority::Behaviour => source::Authority::Behaviour,
    }
}

fn wire_claim(claim: adapter::Claim) -> source::Claim {
    source::Claim {
        kind: wire_claim_kind(claim.kind),
        id: claim.id,
        path: claim.path,
        synopsis: claim.synopsis,
        backing: claim.backing.map(|backing| match backing {
            adapter::Backing::Payload(payload) => source::Backing::Payload(payload),
            adapter::Backing::Path(path) => source::Backing::Path(path),
        }),
    }
}

const fn wire_claim_kind(kind: adapter::ClaimKind) -> source::ClaimKind {
    match kind {
        adapter::ClaimKind::Requirement => source::ClaimKind::Requirement,
        adapter::ClaimKind::Criterion => source::ClaimKind::Criterion,
        adapter::ClaimKind::Section => source::ClaimKind::Section,
    }
}
