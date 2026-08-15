//! GitHub `forge.find` kernel over a local HTTP fixture: typed rows,
//! same-repository head filtering, pagination to exhaustion, the
//! bearer token, and the typed auth / transport outcomes.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use project::seam::{ForgeError, PrState};
use project::vcs::forge::{Config, find};

/// One canned response the fixture serves, matched by page number.
struct Page {
    status: u16,
    body: String,
}

/// One recorded request: path plus the authorization header.
type Seen = Arc<Mutex<Vec<(String, Option<String>)>>>;

/// A minimal single-threaded HTTP fixture: serves one canned page per
/// `page=` query value and records every request's path and
/// authorization header.
struct Fixture {
    base: String,
    seen: Seen,
}

impl Fixture {
    /// A snapshot of the recorded requests.
    fn requests(&self) -> Vec<(String, Option<String>)> {
        self.seen.lock().expect("seen").clone()
    }
}

fn serve(pages: Vec<Page>) -> Fixture {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
    let base = format!("http://{}", listener.local_addr().expect("addr"));
    let seen: Seen = Arc::default();
    let recorder = Arc::clone(&seen);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream);
            let mut request_line = String::new();
            if reader.read_line(&mut request_line).is_err() {
                continue;
            }
            let path = request_line.split_whitespace().nth(1).unwrap_or_default().to_string();
            let mut authorization = None;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
                    break;
                }
                if let Some((name, value)) = line.split_once(':')
                    && name.eq_ignore_ascii_case("authorization")
                {
                    authorization = Some(value.trim().to_string());
                }
            }
            recorder.lock().expect("record").push((path.clone(), authorization));
            let page: usize = path
                .split("&page=")
                .nth(1)
                .and_then(|rest| rest.split('&').next())
                .and_then(|raw| raw.parse().ok())
                .unwrap_or(1);
            let empty = Page {
                status: 200,
                body: "[]".to_string(),
            };
            let served = pages.get(page - 1).unwrap_or(&empty);
            let response = format!(
                "HTTP/1.1 {} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                served.status,
                served.body.len(),
                served.body
            );
            let mut stream = reader.into_inner();
            drop(stream.write_all(response.as_bytes()));
        }
    });
    Fixture { base, seen }
}

fn config(fixture: &Fixture, token: Option<&str>) -> Config {
    Config {
        api_base: fixture.base.clone(),
        token: token.map(str::to_string),
    }
}

fn row(url: &str, state: &str, merged_at: Option<&str>, head_label: &str) -> serde_json::Value {
    serde_json::json!({
        "html_url": url,
        "body": "Emery-Change: demo\nEmery-Change-Digest: sha256:aa",
        "state": state,
        "merged_at": merged_at,
        "merge_commit_sha": merged_at.map(|_| "8e43c0ffee"),
        "base": { "ref": "main" },
        "head": { "label": head_label }
    })
}

#[test]
fn typed_rows() {
    let pages = vec![Page {
        status: 200,
        body: serde_json::json!([
            row(
                "https://github.com/o/r/pull/1",
                "closed",
                Some("2026-08-01T00:00:00Z"),
                "o:change/demo"
            ),
            row("https://github.com/o/r/pull/2", "open", None, "o:change/demo"),
            row("https://github.com/o/r/pull/3", "closed", None, "o:change/demo"),
        ])
        .to_string(),
    }];
    let fixture = serve(pages);
    let pulls = find(&config(&fixture, None), "https://github.com/o/r", "change/demo")
        .expect("find succeeds");
    assert_eq!(pulls.len(), 3);
    assert_eq!(pulls[0].state, PrState::Merged);
    assert_eq!(pulls[0].merge_commit.as_deref(), Some("8e43c0ffee"));
    assert_eq!(pulls[0].merged_at.as_deref(), Some("2026-08-01T00:00:00Z"));
    assert_eq!(pulls[0].base, "main");
    assert_eq!(pulls[1].state, PrState::Open);
    assert!(pulls[1].merge_commit.is_none());
    assert_eq!(pulls[2].state, PrState::Closed);
}

#[test]
fn fork_heads_filtered() {
    let pages = vec![Page {
        status: 200,
        body: serde_json::json!([
            row("https://github.com/o/r/pull/1", "open", None, "o:change/demo"),
            row("https://github.com/o/r/pull/2", "open", None, "fork:change/demo"),
        ])
        .to_string(),
    }];
    let fixture = serve(pages);
    let pulls = find(&config(&fixture, None), "https://github.com/o/r", "change/demo")
        .expect("find succeeds");
    assert_eq!(pulls.len(), 1);
    assert_eq!(pulls[0].url, "https://github.com/o/r/pull/1");
}

#[test]
fn pagination_to_exhaustion() {
    let full: Vec<serde_json::Value> = (0..100)
        .map(|n| row(&format!("https://github.com/o/r/pull/{n}"), "open", None, "o:change/demo"))
        .collect();
    let pages = vec![
        Page {
            status: 200,
            body: serde_json::Value::Array(full).to_string(),
        },
        Page {
            status: 200,
            body: serde_json::json!([row(
                "https://github.com/o/r/pull/100",
                "open",
                None,
                "o:change/demo"
            )])
            .to_string(),
        },
    ];
    let fixture = serve(pages);
    let pulls = find(&config(&fixture, None), "https://github.com/o/r", "change/demo")
        .expect("find succeeds");
    assert_eq!(pulls.len(), 101);
    let seen = fixture.requests();
    assert_eq!(seen.len(), 2, "one request per page, stopped at the short page");
    assert!(seen[0].0.contains("page=1"));
    assert!(seen[1].0.contains("page=2"));
}

#[test]
fn bearer_token_sent() {
    let fixture = serve(vec![Page {
        status: 200,
        body: "[]".to_string(),
    }]);
    find(&config(&fixture, Some("t0ken")), "https://github.com/o/r", "change/demo")
        .expect("find succeeds");
    let seen = fixture.requests();
    assert_eq!(seen[0].1.as_deref(), Some("Bearer t0ken"));
}

#[test]
fn no_token_no_header() {
    let fixture = serve(vec![Page {
        status: 200,
        body: "[]".to_string(),
    }]);
    find(&config(&fixture, None), "https://github.com/o/r", "change/demo").expect("find succeeds");
    let seen = fixture.requests();
    assert_eq!(seen[0].1, None);
}

#[test]
fn auth_status_is_auth_error() {
    let fixture = serve(vec![Page {
        status: 401,
        body: "{}".to_string(),
    }]);
    let err = find(&config(&fixture, None), "https://github.com/o/r", "change/demo")
        .expect_err("401 refuses");
    assert!(matches!(err, ForgeError::Auth(_)), "got {err:?}");
}

#[test]
fn server_error_is_transport() {
    let fixture = serve(vec![Page {
        status: 500,
        body: "{}".to_string(),
    }]);
    let err = find(&config(&fixture, None), "https://github.com/o/r", "change/demo")
        .expect_err("500 refuses");
    assert!(matches!(err, ForgeError::Transport(_)), "got {err:?}");
}

#[test]
fn refused_is_transport() {
    let config = Config {
        api_base: "http://127.0.0.1:1".to_string(),
        token: None,
    };
    let err =
        find(&config, "https://github.com/o/r", "change/demo").expect_err("dead port refuses");
    assert!(matches!(err, ForgeError::Transport(_)), "got {err:?}");
}

mod repository_grammar {
    use super::*;

    #[test]
    fn non_github_refused() {
        let config = Config {
            api_base: "http://unused".to_string(),
            token: None,
        };
        let err =
            find(&config, "https://gitlab.com/o/r", "change/demo").expect_err("non-GitHub refuses");
        assert!(matches!(err, ForgeError::InvalidRequest(_)), "got {err:?}");
    }

    #[test]
    fn malformed_refused() {
        let config = Config {
            api_base: "http://unused".to_string(),
            token: None,
        };
        for repository in ["https://github.com/only-owner", "https://github.com/o/r/extra"] {
            let err = find(&config, repository, "change/demo").expect_err("malformed refuses");
            assert!(matches!(err, ForgeError::InvalidRequest(_)), "{repository}: {err:?}");
        }
    }

    #[test]
    fn dot_git_suffix_stripped() {
        let fixture = serve(vec![Page {
            status: 200,
            body: "[]".to_string(),
        }]);
        find(&config(&fixture, None), "https://github.com/o/r.git", "change/demo")
            .expect("find succeeds");
        let seen = fixture.requests();
        assert!(seen[0].0.starts_with("/repos/o/r/pulls"), "path: {}", seen[0].0);
    }
}

mod token_order {
    use project::vcs::forge::Config;

    /// Pin the token-resolution environment: `GITHUB_TOKEN` plus an
    /// empty `PATH` so `gh auth token` can never answer.
    #[expect(
        unsafe_code,
        reason = "token order reads the process environment; nextest isolates the process"
    )]
    fn pin_env(github_token: &str) {
        // SAFETY: nextest runs each test in its own process.
        unsafe { std::env::set_var("GITHUB_TOKEN", github_token) };
        // SAFETY: same isolated process.
        unsafe { std::env::set_var("PATH", "") };
    }

    #[test]
    fn github_token_env_wins() {
        pin_env(" env-token ");
        let config = Config::github();
        assert_eq!(config.token.as_deref(), Some("env-token"));
    }

    #[test]
    fn empty_env_falls_through() {
        pin_env("  ");
        let config = Config::github();
        assert_eq!(config.token, None, "blank env token falls through; no gh on PATH");
    }
}
