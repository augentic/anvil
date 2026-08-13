//! Bounded HTTPS GET: HTTPS-only, credential-free, no private networks.

use std::io::Read as _;
use std::net::ToSocketAddrs as _;

use error::Error;
use project::binding::{self, Meter, Policy};
use reqwest::blocking::Client;
use reqwest::header::LOCATION;
use reqwest::redirect::Policy as Redirect;

/// Fetch `url` following at most `policy.https_redirects` hops.
///
/// GitHub document pages are rewritten to raw content. Each hop is gated
/// (HTTPS, no credentials, no private-network target) and the body is capped.
///
/// # Errors
///
/// URL-gate diagnostics, `https-fetch-failed`, `https-redirect-limit`,
/// `https-body-limit`, `binding-budget-exhausted`.
pub fn fetch(url: &str, policy: &Policy, meter: &mut Meter) -> Result<Vec<u8>, Error> {
    let url = binding::raw_github(url);
    binding::check_https(&url)?;
    pull(&url, policy, meter, true)
}

/// Resolve `host` and refuse private-network addresses.
///
/// # Errors
///
/// `locator-private-network`, `https-fetch-failed`.
fn resolved_ip(host: &str) -> Result<(), Error> {
    let addrs = (host, 443_u16).to_socket_addrs().map_err(|err| Error::Diag {
        code: "https-fetch-failed",
        detail: format!("failed to resolve `{host}`: {err}"),
    })?;
    for addr in addrs {
        if binding::is_private(addr.ip()) {
            return Err(Error::Diag {
                code: "locator-private-network",
                detail: format!("HTTPS locator host `{host}` resolved to a private network"),
            });
        }
    }
    Ok(())
}

fn pull(start: &str, policy: &Policy, meter: &mut Meter, gate: bool) -> Result<Vec<u8>, Error> {
    let client = Client::builder()
        .redirect(Redirect::none())
        .timeout(std::time::Duration::from_millis(policy.time_ms))
        .build()
        .map_err(|err| fetch_failed(&format!("failed to build HTTPS client: {err}")))?;
    let mut url = start.to_string();
    for hop in 0..=policy.https_redirects {
        meter.time(policy)?;
        if gate {
            binding::check_https(&url)?;
            if let Some(host) = host_of(&url) {
                resolved_ip(&host)?;
            }
        }
        meter.api(policy)?;
        let response = client
            .get(&url)
            .send()
            .map_err(|err| fetch_failed(&format!("HTTPS GET `{url}` failed: {err}")))?;
        if response.status().is_redirection() {
            if hop == policy.https_redirects {
                return Err(Error::Diag {
                    code: "https-redirect-limit",
                    detail: format!(
                        "HTTPS GET exceeded redirect budget ({})",
                        policy.https_redirects
                    ),
                });
            }
            let next =
                response.headers().get(LOCATION).and_then(|value| value.to_str().ok()).ok_or_else(
                    || fetch_failed(&format!("redirect from `{url}` has no Location")),
                )?;
            url = join_redirect(&url, next);
            continue;
        }
        if !response.status().is_success() {
            return Err(fetch_failed(&format!("HTTPS GET `{url}` returned {}", response.status())));
        }
        return read_body(response, policy);
    }
    Err(Error::Diag {
        code: "https-redirect-limit",
        detail: format!("HTTPS GET exceeded redirect budget ({})", policy.https_redirects),
    })
}

fn read_body(mut response: reqwest::blocking::Response, policy: &Policy) -> Result<Vec<u8>, Error> {
    let mut body = Vec::new();
    let mut buf = [0_u8; 8192];
    loop {
        let n =
            response.read(&mut buf).map_err(|err| fetch_failed(&format!("body read: {err}")))?;
        if n == 0 {
            break;
        }
        if body.len().saturating_add(n) > policy.https_body {
            return Err(Error::Diag {
                code: "https-body-limit",
                detail: format!("HTTPS body exceeded budget ({})", policy.https_body),
            });
        }
        body.extend_from_slice(&buf[..n]);
    }
    Ok(body)
}

fn host_of(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let hostport = rest.split('/').next()?;
    let host = hostport.rsplit_once(':').map_or(hostport, |(host, _)| host);
    Some(host.trim_matches(['[', ']']).to_string())
}

fn join_redirect(base: &str, location: &str) -> String {
    if location.starts_with("https://") || location.starts_with("http://") {
        return location.to_string();
    }
    if let Some(scheme_host) = base.splitn(4, '/').take(3).collect::<Vec<_>>().get(2).copied() {
        let origin = if base.starts_with("https://") {
            format!("https://{scheme_host}")
        } else {
            format!("http://{scheme_host}")
        };
        if location.starts_with('/') {
            return format!("{origin}{location}");
        }
        return format!("{origin}/{location}");
    }
    location.to_string()
}

fn fetch_failed(detail: &str) -> Error {
    Error::Diag {
        code: "https-fetch-failed",
        detail: detail.into(),
    }
}

#[cfg(test)]
mod transport {
    use std::io::Write as _;
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    fn serve(hops: Vec<Vec<u8>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            for body in hops {
                let Ok((mut stream, _)) = listener.accept() else { break };
                let mut incoming = [0_u8; 4096];
                drop(std::io::Read::read(&mut stream, &mut incoming));
                drop(stream.write_all(&body));
            }
        });
        format!("http://{addr}")
    }

    fn http_ok(body: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }

    fn http_redirect(to: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .into_bytes()
    }

    #[test]
    fn body_cap() {
        let url = serve(vec![http_ok("hello world")]);
        let policy = Policy {
            https_body: 4,
            ..Policy::standard()
        };
        let err = pull(&url, &policy, &mut Meter::new(), false).expect_err("cap");
        assert!(err.to_string().contains("https-body-limit"), "{err}");
    }

    #[test]
    fn redirect_cap() {
        let url = serve(vec![http_redirect("/next"), http_redirect("/next"), http_ok("done")]);
        let policy = Policy {
            https_redirects: 1,
            ..Policy::standard()
        };
        let err = pull(&url, &policy, &mut Meter::new(), false).expect_err("redirects");
        assert!(err.to_string().contains("https-redirect-limit"), "{err}");
    }
}
