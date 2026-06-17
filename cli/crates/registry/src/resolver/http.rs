//! HTTPS source resolution with per-call timeouts, a 64 MiB body cap,
//! and stream-to-tempfile persistence. See AGENTS.md
//! §"ureq fetch hardening" for the contract every HTTP path must follow.

use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;
use std::{fs, io};

use tempfile::NamedTempFile;
use ureq::ResponseExt;

use crate::error::ExtensionError;
use crate::package::AcquiredBytes;

/// Whole-call cap; covers DNS + connect + headers + body.
const REQUEST_TIMEOUT: Duration = Duration::from_mins(2);
/// TCP + TLS handshake.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
/// Sending request headers / body. WASI tool fetches issue empty bodies, but
/// the explicit cap defends against a peer that stalls during the request.
const SEND_TIMEOUT: Duration = Duration::from_secs(30);
/// Receiving the response body. Generous to allow large WASI components on
/// slow links without breaching the global cap.
const RECV_BODY_TIMEOUT: Duration = Duration::from_mins(1);
/// Receiving the response headers.
const RECV_HEADERS_TIMEOUT: Duration = Duration::from_secs(30);
/// Maximum accepted WASI component download size. Larger payloads abort
/// streaming before they exhaust the cache filesystem.
pub(super) const MAX_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;
pub(super) const STREAM_CHUNK_BYTES: usize = 64 * 1024;
const USER_AGENT: &str =
    concat!("specify-tool/", env!("CARGO_PKG_VERSION"), " (+https://github.com/augentic/specify)");

pub(super) fn download_https(url: &str, dest_hint: &Path) -> Result<AcquiredBytes, ExtensionError> {
    if !url.starts_with("https://") {
        return Err(ExtensionError::Diag {
            code: "tool-resolver",
            detail: format!(
                "tool source `{url}` is invalid: production network sources must use https://; http:// is not supported"
            ),
        });
    }

    let mut response = https_agent().get(url).call().map_err(|err| map_network_error(url, err))?;
    let final_uri = response.get_uri().to_string();
    if !final_uri.starts_with("https://") {
        return Err(ExtensionError::Diag {
            code: "tool-resolver",
            detail: format!(
                "tool source `{url}` is invalid: redirect target must remain https://, got `{final_uri}`"
            ),
        });
    }

    let status = response.status().as_u16();
    if status != 200 {
        return Err(ExtensionError::network_status(url, status));
    }

    // Fail fast when the server advertises a body larger than the cap. Without
    // a Content-Length the cap is enforced while streaming below.
    if let Some(length) = response.body().content_length()
        && length > MAX_RESPONSE_BYTES
    {
        return Err(ExtensionError::network_too_large(url, MAX_RESPONSE_BYTES, Some(length)));
    }

    let temp_parent = dest_hint.parent().ok_or_else(|| {
        ExtensionError::cache_root(format!(
            "tool download destination has no parent: {}",
            dest_hint.display()
        ))
    })?;
    fs::create_dir_all(temp_parent).map_err(|err| {
        ExtensionError::cache_io("create download staging parent", temp_parent, err)
    })?;
    let temp = NamedTempFile::new_in(temp_parent)
        .map_err(|err| ExtensionError::cache_io("create download tempfile", temp_parent, err))?;

    let mut reader = response.body_mut().with_config().limit(MAX_RESPONSE_BYTES + 1).reader();
    let sha256 = stream_to_tempfile(url, &mut reader, &temp)?;
    Ok(AcquiredBytes {
        temp,
        sha256,
        package_metadata: None,
    })
}

fn stream_to_tempfile<R: Read>(
    url: &str, reader: &mut R, temp: &NamedTempFile,
) -> Result<String, ExtensionError> {
    let mut hasher = specify_schema::digest::Hasher::new();
    let mut writer = io::BufWriter::with_capacity(STREAM_CHUNK_BYTES, temp.as_file());
    let mut buf = vec![0_u8; STREAM_CHUNK_BYTES];
    let mut total: u64 = 0;
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(err) => {
                if let Some(ureq_err) = err.get_ref().and_then(|e| e.downcast_ref::<ureq::Error>())
                {
                    return Err(map_streamed_body_error(url, ureq_err));
                }
                return Err(ExtensionError::network_other(url, err.to_string()));
            }
        };
        total = total.saturating_add(n as u64);
        if total > MAX_RESPONSE_BYTES {
            return Err(ExtensionError::network_too_large(url, MAX_RESPONSE_BYTES, Some(total)));
        }
        hasher.update(&buf[..n]);
        writer
            .write_all(&buf[..n])
            .map_err(|err| ExtensionError::cache_io("write download tempfile", temp.path(), err))?;
    }
    writer
        .flush()
        .map_err(|err| ExtensionError::cache_io("flush download tempfile", temp.path(), err))?;
    drop(writer);
    temp.as_file()
        .sync_all()
        .map_err(|err| ExtensionError::cache_io("sync download tempfile", temp.path(), err))?;
    Ok(hasher.finalize_hex())
}

fn https_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_send_request(Some(SEND_TIMEOUT))
        .timeout_send_body(Some(SEND_TIMEOUT))
        .timeout_recv_response(Some(RECV_HEADERS_TIMEOUT))
        .timeout_recv_body(Some(RECV_BODY_TIMEOUT))
        .https_only(true)
        .http_status_as_error(false)
        .user_agent(USER_AGENT)
        .build()
        .into()
}

fn map_network_error(url: &str, err: ureq::Error) -> ExtensionError {
    match err {
        ureq::Error::StatusCode(status) => ExtensionError::network_status(url, status),
        ureq::Error::Timeout(timeout) => ExtensionError::network_timeout(url, timeout.to_string()),
        ureq::Error::BadUri(detail) => ExtensionError::network_malformed(url, detail),
        ureq::Error::Http(err) => ExtensionError::network_malformed(url, err.to_string()),
        ureq::Error::BodyExceedsLimit(limit) => {
            ExtensionError::network_too_large(url, MAX_RESPONSE_BYTES, Some(limit))
        }
        ureq::Error::RequireHttpsOnly(detail) => ExtensionError::Diag {
            code: "tool-resolver",
            detail: format!("tool source `{url}` is invalid: {detail}"),
        },
        err => ExtensionError::network_other(url, err.to_string()),
    }
}

fn map_streamed_body_error(url: &str, err: &ureq::Error) -> ExtensionError {
    match err {
        ureq::Error::Timeout(timeout) => ExtensionError::network_timeout(url, timeout.to_string()),
        ureq::Error::BodyExceedsLimit(limit) => {
            ExtensionError::network_too_large(url, MAX_RESPONSE_BYTES, Some(*limit))
        }
        err => ExtensionError::network_other(url, err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::super::resolve;
    use super::*;
    use crate::error::ExtensionError;
    use crate::manifest::ExtensionSource;
    use crate::test_support::{cache_env, fixed_now, project_scope, scratch_dir, tool};

    #[test]
    fn http_rejected_before_network() {
        let cache_dir = scratch_dir("resolver-http-cache");
        let project_dir = scratch_dir("resolver-http-project");
        let scope = project_scope();
        let declared =
            tool(ExtensionSource::HttpsUri("http://127.0.0.1/tool.wasm".to_string()), None);

        let _env = cache_env(&cache_dir);

        let err = resolve(&scope, &declared, fixed_now(), &project_dir)
            .expect_err("http must be rejected");
        assert!(
            matches!(&err, ExtensionError::Diag { code: "tool-resolver", detail } if detail.contains("https://")),
            "{err}"
        );
    }

    #[test]
    fn malformed_https_url_returns_typed_error() {
        let dest = scratch_dir("resolver-malformed-https").join("module.wasm");
        let err = download_https("https://", &dest).expect_err("malformed URL must fail");
        assert!(
            matches!(
                err,
                ExtensionError::Diag {
                    code: "tool-network-malformed",
                    ..
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn air_gapped_https_error_names_url() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind unused port");
        let addr = listener.local_addr().expect("local addr");
        drop(listener);
        let url = format!("https://{addr}/tool.wasm");
        let dest = scratch_dir("resolver-air-gapped-https").join("module.wasm");
        let err = download_https(&url, &dest).expect_err("closed local port must fail");
        assert!(err.to_string().contains(&url), "{err}");
        assert!(
            matches!(
                err,
                ExtensionError::Diag {
                    code: "tool-network-other" | "tool-network-timeout" | "tool-network-malformed",
                    ..
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn network_error_mapping_variants() {
        let timeout = map_network_error(
            "https://example.test/tool.wasm",
            ureq::Error::Timeout(ureq::Timeout::Global),
        );
        assert!(
            matches!(
                timeout,
                ExtensionError::Diag {
                    code: "tool-network-timeout",
                    ..
                }
            ),
            "{timeout}"
        );

        let too_large = map_streamed_body_error(
            "https://example.test/tool.wasm",
            &ureq::Error::BodyExceedsLimit(1),
        );
        assert!(
            matches!(
                too_large,
                ExtensionError::Diag {
                    code: "tool-network-too-large",
                    ..
                }
            ),
            "{too_large}"
        );
    }

    // `map_network_error` is the funnel that gives every ureq failure a
    // stable kebab-case `code` the JSON envelope exposes. Each arm routes
    // to a different diagnostic code; a `match` reorder or a copy-paste
    // of the wrong constructor would silently relabel an operator-facing
    // failure. Pin the code per variant.
    #[test]
    fn map_network_error_codes_per_variant() {
        let url = "https://example.test/tool.wasm";
        let cases: Vec<(ureq::Error, &str)> = vec![
            (ureq::Error::StatusCode(503), "tool-network-status"),
            (ureq::Error::BadUri("not a uri".to_string()), "tool-network-malformed"),
            (ureq::Error::BodyExceedsLimit(7), "tool-network-too-large"),
            (ureq::Error::RequireHttpsOnly("http downgrade".to_string()), "tool-resolver"),
        ];
        for (err, expected_code) in cases {
            let mapped = map_network_error(url, err);
            let ExtensionError::Diag { code, .. } = mapped else {
                panic!("expected a Diag, got {mapped}");
            };
            assert_eq!(code, expected_code);
        }
    }
}
