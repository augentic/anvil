use artifacts::evidence;
use project::seam;
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

impl From<Lead> for seam::Lead {
    fn from(lead: Lead) -> Self {
        Self {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        }
    }
}

impl From<seam::Lead> for Lead {
    fn from(lead: seam::Lead) -> Self {
        Self {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        }
    }
}

impl From<evidence::AuthorityClass> for Authority {
    fn from(authority: evidence::AuthorityClass) -> Self {
        match authority {
            evidence::AuthorityClass::Intent => Self::Intent,
            evidence::AuthorityClass::Documentation => Self::Documentation,
            evidence::AuthorityClass::Behaviour => Self::Behaviour,
        }
    }
}

impl From<evidence::Claim> for Claim {
    fn from(claim: evidence::Claim) -> Self {
        Self {
            kind: claim.kind.into(),
            id: claim.id,
            path: claim.path,
            synopsis: claim.synopsis,
            backing: claim.backing().map(Backing::from),
        }
    }
}

impl From<evidence::ClaimKind> for ClaimKind {
    fn from(kind: evidence::ClaimKind) -> Self {
        match kind {
            evidence::ClaimKind::Intent => Self::Intent,
            evidence::ClaimKind::Requirement => Self::Requirement,
            evidence::ClaimKind::Criterion => Self::Criterion,
            evidence::ClaimKind::Decision => Self::Decision,
            evidence::ClaimKind::Section => Self::Section,
            evidence::ClaimKind::Diagram => Self::Diagram,
            evidence::ClaimKind::Contract => Self::Contract,
            evidence::ClaimKind::Example => Self::Example,
            evidence::ClaimKind::Excerpt => Self::Excerpt,
            evidence::ClaimKind::Type => Self::Type,
            evidence::ClaimKind::Call => Self::Call,
            evidence::ClaimKind::Region => Self::Region,
            evidence::ClaimKind::Container => Self::Container,
            evidence::ClaimKind::Leaf => Self::Leaf,
        }
    }
}

impl From<evidence::Backing> for Backing {
    fn from(backing: evidence::Backing) -> Self {
        match backing {
            evidence::Backing::Payload(payload) => Self::Payload(payload),
            evidence::Backing::Path(path) => Self::Path(path),
        }
    }
}
