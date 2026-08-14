//! HTTPS URL gate and GitHub document-page rewrite. No network I/O.

use std::net::IpAddr;

use error::Error;

/// Rewrite a GitHub document page to raw content; other URLs pass through.
#[must_use]
pub fn raw_github(url: &str) -> String {
    let Some(rest) = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("https://www.github.com/"))
    else {
        return url.to_string();
    };
    let mut parts = rest.splitn(4, '/');
    let (Some(owner), Some(repo), Some(kind), Some(tail)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return url.to_string();
    };
    if kind != "blob" && kind != "raw" {
        return url.to_string();
    }
    format!("https://raw.githubusercontent.com/{owner}/{repo}/{tail}")
}

/// Refuse non-HTTPS, credentials, localhost, and private-network literals.
///
/// # Errors
///
/// `locator-http-unsupported`, `locator-credentials-forbidden`,
/// `locator-private-network`, `locator-malformed`.
pub fn check(url: &str) -> Result<(), Error> {
    let url = raw_github(url);
    if url.starts_with("http://") {
        return Err(Error::Diag {
            code: "locator-http-unsupported",
            detail: "remote locators require HTTPS".into(),
        });
    }
    let Some(rest) = url.strip_prefix("https://") else {
        return Err(Error::Diag {
            code: "locator-malformed",
            detail: format!("HTTPS locator `{url}` is not an https URL"),
        });
    };
    let hostport = rest.split('/').next().unwrap_or("");
    if hostport.is_empty() {
        return Err(Error::Diag {
            code: "locator-malformed",
            detail: "HTTPS locator has no host".into(),
        });
    }
    if hostport.contains('@') {
        return Err(Error::Diag {
            code: "locator-credentials-forbidden",
            detail: "remote locators must not contain credentials".into(),
        });
    }
    let host = hostport.rsplit_once(':').map_or(hostport, |(host, _)| host);
    let host = host.trim_matches(['[', ']']);
    if is_local_name(host) || host.parse::<IpAddr>().is_ok_and(is_private) {
        return Err(Error::Diag {
            code: "locator-private-network",
            detail: format!("HTTPS locator host `{host}` targets a private network"),
        });
    }
    Ok(())
}

/// Loopback, RFC1918, link-local, ULA, unspecified, and IPv4-mapped forms.
#[must_use]
pub fn is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || matches!(v4.octets(), [100, n, ..] if (64..128).contains(&n))
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                || v6.is_unspecified()
                || v6.to_ipv4_mapped().is_some_and(|v4| is_private(IpAddr::V4(v4)))
        }
    }
}

fn is_local_name(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    host.eq_ignore_ascii_case("localhost")
        || host.eq_ignore_ascii_case("localhost.")
        || host.to_ascii_lowercase().ends_with(".localhost")
}
