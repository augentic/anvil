//! Requirement provenance
//!
//! Turns every source's requirement claims into the provenance of each
//! requirement — the rows `spec.md` is built on. Which claims across sources
//! describe one requirement, and which of them agree, is a judgement: the
//! model answers it as one partition — claims into requirements, each
//! requirement's claims into agreeing classes — over a deterministic floor
//! that pre-merges byte-equal ids. The engine validates the partition, then
//! derives everything else from it and the closed authority ranking: the
//! subject, the status, the winner and losers, and whether any acceptance
//! criterion covers the requirement.
//!
//! Authority is withheld from the request, so the answer cannot be steered
//! toward a winner; a run over one source never asks at all.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use emery_source::types::{Authority, ClaimKind};
use omnia_guest::{Error, Model};
use schemars::JsonSchema;
use serde::Deserialize;

use super::extract::SourceSet;
use super::judgment::{Findings, Question};
use crate::artifact::Status;

const PROSE: &[&str] = &["synthesis/grouping.md"];

/// Derives the provenance of every requirement in `sets`, asking the model
/// to group the claims on any run over two or more sources.
///
/// # Errors
///
/// Returns the model failure or the exhausted grouping findings.
pub async fn derive<M: Model>(model: &M, sets: &[SourceSet]) -> Result<Vec<Provenance>, Error> {
    if sets.len() < 2 {
        return Ok(floor(sets));
    }

    let claims = Claims::collect(sets);
    tracing::info!("grouping requirement claims");
    let question = Question::<Grouping>::new("grouping", PROSE);
    let grouping = question.ask(model, &claims.prompt(), |grouping| claims.check(grouping)).await?;

    Ok(claims.rows(&grouping))
}

/// The provenance the floor alone derives: byte-equal ids are one
/// requirement, whitespace-equal statements one class. What a run over one
/// source gets.
#[must_use]
pub fn floor(sets: &[SourceSet]) -> Vec<Provenance> {
    let claims = Claims::collect(sets);
    claims.rows(&claims.floor())
}

/// A partition of every requirement claim into requirements, each carrying a
/// partition of its claims into agreeing classes.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery grouping answer")]
pub struct Grouping {
    /// One entry per requirement.
    pub groups: Vec<Group>,
}

/// The claims of one requirement and how they agree.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// Indices of every claim describing this requirement.
    pub claims: Vec<usize>,
    /// A partition of `claims`: each class holds claims that say the same
    /// thing.
    pub classes: Vec<Vec<usize>>,
}

/// The provenance of one requirement — the row `spec.md` renders it as: the
/// subject it is headed with, its status, whether an acceptance criterion
/// covers it, and its contributors in agreeing classes, the winning class
/// first.
#[derive(Debug, Clone)]
pub struct Provenance {
    subject: String,
    status: Status,
    covered: bool,
    classes: Vec<Vec<Contributor>>,
}

impl Provenance {
    // Highest authority first, binding order within a class; one class
    // agrees, a unique top authority wins divergence, top-authority peers
    // in different classes conflict; an uncovered agreed row is unknown.
    fn of(mut classes: Vec<Vec<Contributor>>, criteria: &[&str]) -> Self {
        for class in &mut classes {
            class.sort_by_key(|member| (member.authority.rank(), member.index));
        }
        classes.sort_by_key(|class| (class[0].authority.rank(), class[0].index));

        let top = classes[0][0].authority.rank();
        let covered =
            classes.iter().flatten().any(|member| criteria.iter().any(|id| covers(id, &member.id)));
        let status = match classes.len() {
            1 if covered => Status::Agreed,
            1 => Status::Unknown,
            _ if classes.iter().skip(1).all(|class| class[0].authority.rank() != top) => {
                Status::Divergence
            }
            _ => Status::Conflict,
        };

        Self {
            subject: classes[0][0].id.clone(),
            status,
            covered,
            classes,
        }
    }

    /// The heading name: the top contributor's claim id.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The `Status:` value.
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Whether any criterion claim covers the requirement.
    #[must_use]
    pub const fn covered(&self) -> bool {
        self.covered
    }

    /// The agreeing classes, the winning class first.
    #[must_use]
    pub fn classes(&self) -> &[Vec<Contributor>] {
        &self.classes
    }

    /// Every contributing source key, highest authority first and binding
    /// order within an authority.
    pub fn sources(&self) -> impl Iterator<Item = &str> {
        let mut members: Vec<&Contributor> = self.classes.iter().flatten().collect();
        members.sort_by_key(|member| (member.authority.rank(), member.index));
        members.into_iter().map(|member| member.source.as_str())
    }
}

/// One source's claim in a requirement row.
#[derive(Debug, Clone)]
pub struct Contributor {
    /// The binding key.
    pub source: String,
    /// The source's authority class.
    pub authority: Authority,
    /// The claim id, which may differ from the row's subject.
    pub id: String,
    /// The claim's `statement` extra.
    pub statement: String,
    // Position in binding order, the tie-break within an authority.
    index: usize,
}

// Every requirement claim in binding order, and every criterion id.
struct Claims<'a> {
    requirements: Vec<Indexed<'a>>,
    criteria: Vec<&'a str>,
}

struct Indexed<'a> {
    source: &'a str,
    authority: Authority,
    id: &'a str,
    statement: String,
    synopsis: Option<&'a str>,
}

impl<'a> Claims<'a> {
    fn collect(sets: &'a [SourceSet]) -> Self {
        let mut requirements = Vec::new();
        let mut criteria = Vec::new();
        for set in sets {
            for claim in &set.claims {
                let Some(id) = claim.id.as_deref() else { continue };
                match claim.kind {
                    ClaimKind::Requirement => requirements.push(Indexed {
                        source: &set.key,
                        authority: set.authority,
                        id,
                        statement: claim.statement(),
                        synopsis: claim.synopsis.as_deref(),
                    }),
                    ClaimKind::Criterion => criteria.push(id),
                    _ => {}
                }
            }
        }

        Self {
            requirements,
            criteria,
        }
    }

    // The grouping request: every claim indexed, authority withheld, the
    // floor stated.
    fn prompt(&self) -> String {
        let mut prompt = String::from(
            "Group the requirement claims.\n\n\
             ## Requirement claims (index, source, id, statement, synopsis)\n\n",
        );

        for (index, claim) in self.requirements.iter().enumerate() {
            let synopsis = claim.synopsis.unwrap_or("-");
            let _ = writeln!(
                prompt,
                "- {index} `{source}` `{id}` — {statement} — {synopsis}",
                source = claim.source,
                id = claim.id,
                statement = claim.statement,
            );
        }

        prompt.push_str("\n## Floor\n\n");
        let floor = self.floor();
        let merged: Vec<&Group> =
            floor.groups.iter().filter(|group| group.claims.len() > 1).collect();
        if merged.is_empty() {
            prompt.push_str("No two claims share an id; every grouping is your judgement.\n");
        }

        for group in merged {
            let id = self.requirements[group.claims[0]].id;
            let indices =
                group.claims.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ");
            let _ = writeln!(
                prompt,
                "- claims {indices} share the id `{id}` and are pre-merged; an answer that splits \
                 them across groups is refused."
            );
        }

        prompt.push_str(
            "\nAnswer with every index in exactly one group, and every group's claims in exactly \
             one agreeing class.\n",
        );

        prompt
    }

    // The deterministic floor: byte-equal ids are one group, and within a
    // group whitespace-equal statements are one class.
    fn floor(&self) -> Grouping {
        let mut groups: Vec<(&str, Group)> = Vec::new();
        for (index, claim) in self.requirements.iter().enumerate() {
            let position = groups.iter().position(|(id, _)| *id == claim.id).unwrap_or_else(|| {
                groups.push((claim.id, Group::default()));
                groups.len() - 1
            });
            let group = &mut groups[position].1;
            group.claims.push(index);
            let statement = normalise(&claim.statement);
            let class = group
                .classes
                .iter_mut()
                .find(|class| normalise(&self.requirements[class[0]].statement) == statement);

            match class {
                Some(class) => class.push(index),
                None => group.classes.push(vec![index]),
            }
        }

        Grouping {
            groups: groups.into_iter().map(|(_, group)| group).collect(),
        }
    }

    // Both levels are partitions, and no byte-equal-id pair is split.
    fn check(&self, grouping: &Grouping) -> Result<(), Findings> {
        let mut findings = Vec::new();
        let count = self.requirements.len();
        let mut placed: BTreeMap<usize, usize> = BTreeMap::new();

        for (position, group) in grouping.groups.iter().enumerate() {
            if group.claims.is_empty() {
                findings.push(format!("- group {position} has no claims"));
            }

            for &index in &group.claims {
                if index >= count {
                    findings.push(format!("- group {position}: claim {index} does not exist"));
                } else if placed.insert(index, position).is_some() {
                    findings.push(format!("- claim {index} appears in more than one group"));
                }
            }

            let members: BTreeSet<usize> = group.claims.iter().copied().collect();
            let mut classed = BTreeSet::new();
            for class in &group.classes {
                if class.is_empty() {
                    findings.push(format!("- group {position} has an empty class"));
                }

                for &index in class {
                    if !members.contains(&index) {
                        findings.push(format!(
                            "- group {position}: class member {index} is not one of its claims"
                        ));
                    } else if !classed.insert(index) {
                        findings.push(format!(
                            "- group {position}: claim {index} appears in more than one class"
                        ));
                    }
                }
            }

            for index in members.difference(&classed) {
                findings.push(format!("- group {position}: claim {index} is in no class"));
            }
        }

        for index in (0..count).filter(|index| !placed.contains_key(index)) {
            findings.push(format!("- claim {index} is in no group"));
        }

        // The floor: byte-equal ids may not be split across groups.
        let mut by_id: BTreeMap<&str, BTreeSet<usize>> = BTreeMap::new();
        for (index, claim) in self.requirements.iter().enumerate() {
            if let Some(position) = placed.get(&index) {
                by_id.entry(claim.id).or_default().insert(*position);
            }
        }

        for (id, positions) in by_id {
            if positions.len() > 1 {
                findings.push(format!("- claims sharing the id `{id}` are split across groups"));
            }
        }

        if findings.is_empty() { Ok(()) } else { Err(findings) }
    }

    // Rows in first-seen order of each group's earliest claim.
    fn rows(&self, grouping: &Grouping) -> Vec<Provenance> {
        let mut groups: Vec<(usize, Vec<Vec<Contributor>>)> = grouping
            .groups
            .iter()
            .map(|group| {
                let first = group.claims.iter().copied().min().unwrap_or_default();
                let classes = group
                    .classes
                    .iter()
                    .map(|class| class.iter().map(|&index| self.contributor(index)).collect())
                    .collect();
                (first, classes)
            })
            .collect();
        groups.sort_by_key(|(first, _)| *first);
        groups.into_iter().map(|(_, classes)| Provenance::of(classes, &self.criteria)).collect()
    }

    fn contributor(&self, index: usize) -> Contributor {
        let claim = &self.requirements[index];

        Contributor {
            source: claim.source.to_string(),
            authority: claim.authority,
            id: claim.id.to_string(),
            statement: claim.statement.clone(),
            index,
        }
    }
}

// A criterion covers a requirement when it is that claim id or a dotted
// child of it: `session.timeout.idle` covers `session.timeout`.
fn covers(criterion: &str, requirement: &str) -> bool {
    criterion.strip_prefix(requirement).is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
}

// Whitespace-collapsed text, so a reflowed statement still matches.
pub fn normalise(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
