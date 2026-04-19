//! Registry queries for `vectis update-versions`.
//!
//! Every function here performs a single HTTP request and returns a
//! parsed string (a version number) or a structured error. We use
//! `ureq` for the transport (small, sync, no tokio) and `roxmltree`
//! for the one XML surface that matters (Google Maven).
//!
//! ## Source map
//!
//! | Registry          | Host                               | Format | Consumers                                          |
//! |-------------------|------------------------------------|--------|----------------------------------------------------|
//! | crates.io         | `crates.io/api/v1/crates/<name>`   | JSON   | crux_*, facet, facet_generate, serde, serde_json, uniffi, cargo-swift, cargo-deny, cargo-vet |
//! | Google Maven      | `maven.google.com/<group-path>`    | XML    | compose-bom, kotlin (stdlib), AGP                  |
//! | Maven Central     | `search.maven.org/solrsearch`      | JSON   | koin (io.insert-koin:koin-bom), ktor (io.ktor:ktor-bom) |
//! | GitHub releases   | `api.github.com/repos/<o/r>/releases/latest` | JSON | xcodegen                                  |
//!
//! All queries carry a short timeout so a flaky mirror cannot hang the
//! CLI indefinitely. On any network/parse error we return
//! `VectisError::Internal` with the query context; the orchestrator
//! decides whether to fall back to the current pin or abort.

use std::time::Duration;

use serde::Deserialize;

use crate::error::VectisError;

/// User-Agent header sent on every request so rate-limit diagnostics
/// can identify the caller. crates.io explicitly requires a UA.
const USER_AGENT: &str = concat!(
    "vectis-cli/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/augentic/specify)"
);

/// Per-request timeout. Registry calls should resolve in well under a
/// second in steady state; anything slower is a sign of a flaky mirror
/// or an outage and we prefer failing fast over hanging an interactive
/// developer session.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Build a shared ureq agent with sensible timeouts + our UA.
fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
}

/// Result of a latest-version query: a plain version string plus the
/// URL we resolved against so error messages can attribute failures.
#[derive(Debug, Clone)]
pub struct VersionHit {
    pub version: String,
    /// URL the result came from; retained for future diagnostics even
    /// though the orchestrator doesn't surface it in the JSON today.
    #[allow(dead_code)]
    pub source: String,
}

// -------------------------------------------------------------------
// crates.io
// -------------------------------------------------------------------

/// Shape of `GET /api/v1/crates/<name>` we actually use.
#[derive(Debug, Deserialize)]
struct CratesIoCrateResponse {
    #[serde(rename = "crate")]
    krate: CratesIoCrate,
}

#[derive(Debug, Deserialize)]
struct CratesIoCrate {
    #[serde(rename = "max_stable_version")]
    max_stable_version: Option<String>,
    #[serde(rename = "max_version")]
    max_version: String,
}

/// Shape of `GET /api/v1/crates/<name>/<version>/dependencies`.
#[derive(Debug, Deserialize)]
struct CratesIoDepsResponse {
    dependencies: Vec<CratesIoDep>,
}

#[derive(Debug, Deserialize)]
pub struct CratesIoDep {
    #[serde(rename = "crate_id")]
    pub crate_id: String,
    pub req: String,
    pub kind: String,
}

/// Query the latest *stable* version for `crate_name` on crates.io.
///
/// Prefers `max_stable_version`; falls back to `max_version` when the
/// crate only has pre-release releases (which should be rare for the
/// crates we query).
pub fn crates_io_latest_stable(crate_name: &str) -> Result<VersionHit, VectisError> {
    let url = format!("https://crates.io/api/v1/crates/{crate_name}");
    let body: CratesIoCrateResponse = agent()
        .get(&url)
        .call()
        .map_err(|e| query_error(&url, &e.to_string()))?
        .into_json()
        .map_err(|e| query_error(&url, &e.to_string()))?;
    let version = body
        .krate
        .max_stable_version
        .unwrap_or(body.krate.max_version);
    Ok(VersionHit {
        version,
        source: url,
    })
}

/// Query the `[dependencies]` block of a specific crate version and
/// return them verbatim (caller filters by `crate_id` / `kind`).
pub fn crates_io_dependencies(
    crate_name: &str,
    version: &str,
) -> Result<Vec<CratesIoDep>, VectisError> {
    let url = format!("https://crates.io/api/v1/crates/{crate_name}/{version}/dependencies");
    let body: CratesIoDepsResponse = agent()
        .get(&url)
        .call()
        .map_err(|e| query_error(&url, &e.to_string()))?
        .into_json()
        .map_err(|e| query_error(&url, &e.to_string()))?;
    Ok(body.dependencies)
}

/// Convenience: find a single normal dep by `crate_id` and return its
/// `req` string (e.g. `"=0.31.0"`, `"^1.0"`).
pub fn crates_io_normal_dep_req(
    crate_name: &str,
    version: &str,
    dep_name: &str,
) -> Result<String, VectisError> {
    let deps = crates_io_dependencies(crate_name, version)?;
    deps.into_iter()
        .find(|d| d.kind == "normal" && d.crate_id == dep_name)
        .map(|d| d.req)
        .ok_or_else(|| VectisError::Internal {
            message: format!(
                "crates.io: {crate_name}@{version} has no normal dep on {dep_name:?}"
            ),
        })
}

// -------------------------------------------------------------------
// Google Maven
// -------------------------------------------------------------------

/// Query the `<versioning><latest>` element of a Google Maven
/// `maven-metadata.xml` for the given group + artifact. Filters to
/// stable versions (no `-alpha`, `-beta`, `-rc`, `-dev`).
pub fn google_maven_latest_stable(
    group_id: &str,
    artifact_id: &str,
) -> Result<VersionHit, VectisError> {
    let group_path = group_id.replace('.', "/");
    let url = format!("https://maven.google.com/{group_path}/{artifact_id}/maven-metadata.xml");
    let body: String = agent()
        .get(&url)
        .call()
        .map_err(|e| query_error(&url, &e.to_string()))?
        .into_string()
        .map_err(|e| query_error(&url, &e.to_string()))?;
    let doc = roxmltree::Document::parse(&body)
        .map_err(|e| query_error(&url, &format!("roxmltree parse: {e}")))?;
    let mut versions: Vec<String> = doc
        .descendants()
        .filter(|n| n.has_tag_name("version"))
        .filter_map(|n| n.text().map(str::to_string))
        .filter(|v| is_stable_version(v))
        .collect();
    if versions.is_empty() {
        return Err(VectisError::Internal {
            message: format!("google maven: no stable versions at {url}"),
        });
    }
    versions.sort_by(|a, b| version_cmp(a, b));
    let version = versions.pop().unwrap();
    Ok(VersionHit {
        version,
        source: url,
    })
}

// -------------------------------------------------------------------
// Maven Central (solrsearch)
// -------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct MavenCentralResponse {
    response: MavenCentralInner,
}

#[derive(Debug, Deserialize)]
struct MavenCentralInner {
    docs: Vec<MavenCentralDoc>,
}

#[derive(Debug, Deserialize)]
struct MavenCentralDoc {
    v: String,
}

/// Query Maven Central solrsearch for stable versions of a given GAV,
/// return the highest stable one.
pub fn maven_central_latest_stable(
    group_id: &str,
    artifact_id: &str,
) -> Result<VersionHit, VectisError> {
    let query = format!("g:{group_id} AND a:{artifact_id}");
    let encoded_query = url_encode(&query);
    let url = format!(
        "https://search.maven.org/solrsearch/select?q={encoded_query}&core=gav&rows=40&wt=json"
    );
    let body: MavenCentralResponse = agent()
        .get(&url)
        .call()
        .map_err(|e| query_error(&url, &e.to_string()))?
        .into_json()
        .map_err(|e| query_error(&url, &e.to_string()))?;
    let mut versions: Vec<String> = body
        .response
        .docs
        .into_iter()
        .map(|d| d.v)
        .filter(|v| is_stable_version(v))
        .collect();
    if versions.is_empty() {
        return Err(VectisError::Internal {
            message: format!("maven central: no stable versions for {group_id}:{artifact_id}"),
        });
    }
    versions.sort_by(|a, b| version_cmp(a, b));
    let version = versions.pop().unwrap();
    Ok(VersionHit {
        version,
        source: url,
    })
}

// -------------------------------------------------------------------
// GitHub releases
// -------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
}

/// Query `releases/latest` for the given `owner/repo` and return the
/// tag with any leading `v` stripped.
pub fn github_latest_release(owner: &str, repo: &str) -> Result<VersionHit, VectisError> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let body: GithubRelease = agent()
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| query_error(&url, &e.to_string()))?
        .into_json()
        .map_err(|e| query_error(&url, &e.to_string()))?;
    let version = body.tag_name.trim_start_matches('v').to_string();
    Ok(VersionHit {
        version,
        source: url,
    })
}

// -------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------

/// Minimal URL-encoder for our single-parameter Maven Central query.
/// Space becomes `+`, other reserved characters become `%XX`. Keeps us
/// free of the `url` crate at the dependency level.
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Is this version string a "stable" release (no pre-release tail)?
///
/// We refuse anything with a `-` in it (`-alpha`, `-beta`, `-rc`,
/// `-dev`, `-M1`, ...). Also refuses versions containing `SNAPSHOT`.
/// Chunk 11 uses this as a strict gate so `update-versions` never
/// proposes a pre-release pin -- the template-updater skill (chunk 13)
/// will opt into pre-releases explicitly when needed.
pub fn is_stable_version(v: &str) -> bool {
    !v.contains('-') && !v.to_ascii_uppercase().contains("SNAPSHOT") && !v.is_empty()
}

/// Compare two version strings lexicographically on their numeric
/// components. Safe against non-semver shapes: any non-integer
/// component sorts as zero so the function always terminates.
/// Adequate for picking a maximum; not a full semver parser.
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<u64> = a.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    let b_parts: Vec<u64> = b.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    let len = a_parts.len().max(b_parts.len());
    for i in 0..len {
        let av = a_parts.get(i).copied().unwrap_or(0);
        let bv = b_parts.get(i).copied().unwrap_or(0);
        match av.cmp(&bv) {
            std::cmp::Ordering::Equal => continue,
            ord => return ord,
        }
    }
    std::cmp::Ordering::Equal
}

fn query_error(url: &str, message: &str) -> VectisError {
    VectisError::Internal {
        message: format!("registry query failed ({url}): {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_stable_rejects_common_prerelease_tails() {
        for pre in [
            "1.0.0-alpha",
            "1.0.0-beta1",
            "1.0.0-rc.1",
            "1.0.0-dev",
            "2.3.0-M1",
            "3.1.0-SNAPSHOT",
            "3.1.0-snapshot",
            "",
        ] {
            assert!(!is_stable_version(pre), "should reject {pre:?}");
        }
    }

    #[test]
    fn is_stable_accepts_release_shapes() {
        for ok in ["0.31.0", "2026.01.01", "3.4.0", "8.13", "8.13.2", "4.1.1"] {
            assert!(is_stable_version(ok), "should accept {ok:?}");
        }
    }

    #[test]
    fn version_cmp_numeric_components_win() {
        assert_eq!(version_cmp("1.10.0", "1.9.0"), std::cmp::Ordering::Greater);
        assert_eq!(version_cmp("1.9.0", "1.10.0"), std::cmp::Ordering::Less);
        assert_eq!(version_cmp("1.9", "1.9.0"), std::cmp::Ordering::Equal);
        assert_eq!(version_cmp("2.0.0", "2.0.0"), std::cmp::Ordering::Equal);
        assert_eq!(
            version_cmp("2026.01.01", "2025.12.31"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn url_encode_handles_space_and_colon() {
        // Maven Central queries use `g:X AND a:Y` -- space becomes `+`,
        // `:` becomes `%3A`.
        assert_eq!(url_encode("g:foo AND a:bar"), "g%3Afoo+AND+a%3Abar");
    }

    #[test]
    fn url_encode_passes_through_unreserved() {
        assert_eq!(url_encode("abcXYZ123._-~"), "abcXYZ123._-~");
    }
}
