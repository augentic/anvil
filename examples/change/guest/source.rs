use testkit::adapter;

use crate::Adapter;
use crate::bindings::exports::specify::adapter::source::{
    AdapterId, AdapterMetadata, Authority, Backing, Claim, ClaimKind, Error, Evidence, Guest, Lead,
};

impl Guest for Adapter {
    fn metadata(_id: AdapterId) -> AdapterMetadata {
        AdapterMetadata { specify_floor: None }
    }

    async fn survey(id: AdapterId) -> Result<Vec<Lead>, Error> {
        let leads = adapter::survey(&id).map_err(Error::from)?;
        Ok(leads.into_iter().map(Lead::from).collect())
    }

    async fn extract(id: AdapterId, lead: Lead) -> Result<Evidence, Error> {
        let evidence = adapter::extract(&id, &lead.into()).map_err(Error::from)?;

        Ok(Evidence {
            authority: evidence.authority.into(),
            claims: evidence.claims.into_iter().map(Claim::from).collect(),
        })
    }
}

impl From<Lead> for adapter::Lead {
    fn from(lead: Lead) -> Self {
        Self {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        }
    }
}

impl From<adapter::Lead> for Lead {
    fn from(lead: adapter::Lead) -> Self {
        Self {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        }
    }
}

impl From<adapter::Authority> for Authority {
    fn from(authority: adapter::Authority) -> Self {
        match authority {
            adapter::Authority::Intent => Authority::Intent,
            adapter::Authority::Documentation => Authority::Documentation,
            adapter::Authority::Behaviour => Authority::Behaviour,
        }
    }
}

impl From<adapter::Claim> for Claim {
    fn from(claim: adapter::Claim) -> Self {
        Self {
            kind: claim.kind.into(),
            id: claim.id,
            path: claim.path,
            synopsis: claim.synopsis,
            backing: claim.backing.map(Backing::from),
        }
    }
}

impl From<adapter::ClaimKind> for ClaimKind {
    fn from(kind: adapter::ClaimKind) -> Self {
        match kind {
            adapter::ClaimKind::Requirement => Self::Requirement,
            adapter::ClaimKind::Criterion => Self::Criterion,
            adapter::ClaimKind::Section => Self::Section,
        }
    }
}

impl From<adapter::Backing> for Backing {
    fn from(backing: adapter::Backing) -> Self {
        match backing {
            adapter::Backing::Payload(payload) => Self::Payload(payload),
            adapter::Backing::Path(path) => Self::Path(path),
        }
    }
}
