//! Workstation toolchain detection (RFC-6 § Prerequisite Detection).
//!
//! Every subcommand calls [`check`] before doing any other work. The set of
//! tools verified is scoped to the [`AssemblyKind`]s the command will touch:
//! `init` checks core plus whatever `--shells` lists, `add-shell` checks core
//! plus the named platform, `verify` checks core plus every assembly directory
//! present on disk, and `update-versions` checks core only (or every assembly
//! when `--verify` is passed).
//!
//! When any tool is missing the function returns
//! [`VectisError::MissingPrerequisites`], which the dispatcher renders as the
//! RFC's structured `missing_prerequisites` JSON shape and exits with code 2.
//! No partial work is performed -- a missing toolchain is a hard stop, not a
//! warning.

use crate::error::{MissingTool, VectisError};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

/// Which assembly a tool belongs to. Each subcommand selects a subset of these
/// before calling [`check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssemblyKind {
    Core,
    Ios,
    Android,
}

impl AssemblyKind {
    /// Tag string used in the JSON error payload (`"core"`, `"ios"`,
    /// `"android"`).
    pub fn tag(self) -> &'static str {
        match self {
            AssemblyKind::Core => "core",
            AssemblyKind::Ios => "ios",
            AssemblyKind::Android => "android",
        }
    }
}

/// Run prerequisite checks for the given assemblies.
///
/// Every tool whose assembly is in `assemblies` is checked. On any failure all
/// failing tools are collected and returned as a single
/// [`VectisError::MissingPrerequisites`] -- the user gets one report listing
/// everything to install rather than fixing them one at a time.
pub fn check(assemblies: &[AssemblyKind]) -> Result<(), VectisError> {
    let needed: HashSet<AssemblyKind> = assemblies.iter().copied().collect();
    let mut missing = Vec::new();

    for tool in all_tools() {
        if !needed.contains(&tool.assembly) {
            continue;
        }
        if run_check(&tool.check).is_err() {
            missing.push(MissingTool {
                tool: tool.name.into(),
                assembly: tool.assembly.tag().into(),
                check: tool.check_display.into(),
                install: tool.install.into(),
            });
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(VectisError::MissingPrerequisites {
            missing,
            message: "Install the missing tools above and re-run the command.".into(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tool registry
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Tool {
    /// Stable identifier reported in the JSON payload.
    name: &'static str,
    assembly: AssemblyKind,
    /// Human-readable check command (matches the RFC's "How to check" column).
    check_display: &'static str,
    /// Install hint shown to the user.
    install: &'static str,
    /// How to actually verify the tool is present.
    check: ToolCheck,
}

#[derive(Debug)]
enum ToolCheck {
    /// Run a command and require it to exit successfully. If `min_version` is
    /// set, also extract a `M.m[.p]` token from combined stdout+stderr and
    /// require it to be at least that version.
    Cmd {
        program: &'static str,
        args: &'static [&'static str],
        min_version: Option<Version>,
    },
    /// Environment variable must be set; if `must_exist`, its value must
    /// resolve to an existing directory.
    Env {
        var: &'static str,
        must_exist: bool,
    },
    /// `rustup target list --installed` output must contain every listed
    /// target.
    RustupTargets(&'static [&'static str]),
    /// `$ANDROID_HOME/ndk` must exist and contain at least one entry.
    AndroidNdk,
}

fn all_tools() -> &'static [Tool] {
    TOOLS
}

// Sourced verbatim from RFC-6 § Workstation Requirements.
static TOOLS: &[Tool] = &[
        // ---- Core --------------------------------------------------------
        Tool {
            name: "rustup",
            assembly: AssemblyKind::Core,
            check_display: "rustup show active-toolchain",
            install: "https://rustup.rs",
            check: ToolCheck::Cmd {
                program: "rustup",
                args: &["show", "active-toolchain"],
                min_version: None,
            },
        },
        Tool {
            name: "cargo-deny",
            assembly: AssemblyKind::Core,
            check_display: "cargo deny --version",
            install: "cargo install cargo-deny",
            check: ToolCheck::Cmd {
                program: "cargo",
                args: &["deny", "--version"],
                min_version: None,
            },
        },
        Tool {
            name: "cargo-vet",
            assembly: AssemblyKind::Core,
            check_display: "cargo vet --version",
            install: "cargo install cargo-vet",
            check: ToolCheck::Cmd {
                program: "cargo",
                args: &["vet", "--version"],
                min_version: None,
            },
        },
        // ---- iOS ---------------------------------------------------------
        Tool {
            name: "xcode",
            assembly: AssemblyKind::Ios,
            check_display: "xcode-select -p",
            install: "Install Xcode + Command Line Tools from the Mac App Store",
            check: ToolCheck::Cmd {
                program: "xcode-select",
                args: &["-p"],
                min_version: None,
            },
        },
        Tool {
            name: "xcodegen",
            assembly: AssemblyKind::Ios,
            check_display: "xcodegen --version",
            install: "brew install xcodegen",
            check: ToolCheck::Cmd {
                program: "xcodegen",
                args: &["--version"],
                min_version: None,
            },
        },
        Tool {
            name: "cargo-swift",
            assembly: AssemblyKind::Ios,
            check_display: "cargo swift --version",
            install: "cargo install cargo-swift",
            check: ToolCheck::Cmd {
                program: "cargo",
                args: &["swift", "--version"],
                min_version: None,
            },
        },
        Tool {
            name: "xcbeautify",
            assembly: AssemblyKind::Ios,
            check_display: "xcbeautify --version",
            install: "brew install xcbeautify",
            check: ToolCheck::Cmd {
                program: "xcbeautify",
                args: &["--version"],
                min_version: None,
            },
        },
        // ---- Android -----------------------------------------------------
        Tool {
            name: "android-sdk",
            assembly: AssemblyKind::Android,
            check_display: "echo $ANDROID_HOME",
            install: "Install Android Studio from https://developer.android.com/studio",
            check: ToolCheck::Env {
                var: "ANDROID_HOME",
                must_exist: true,
            },
        },
        Tool {
            name: "java",
            assembly: AssemblyKind::Android,
            check_display: "java --version",
            install:
                "Install JDK 21+ from https://adoptium.net or `brew install openjdk@21`",
            check: ToolCheck::Cmd {
                program: "java",
                args: &["--version"],
                min_version: Some(Version::new(21, 0, 0)),
            },
        },
        Tool {
            name: "rustup-android-targets",
            assembly: AssemblyKind::Android,
            check_display: "rustup target list --installed",
            install: "rustup target add aarch64-linux-android armv7-linux-androideabi \
                      x86_64-linux-android i686-linux-android",
            check: ToolCheck::RustupTargets(&[
                "aarch64-linux-android",
                "armv7-linux-androideabi",
                "x86_64-linux-android",
                "i686-linux-android",
            ]),
        },
        Tool {
            name: "android-ndk",
            assembly: AssemblyKind::Android,
            check_display: "ls $ANDROID_HOME/ndk/",
            install: "Install the NDK via Android Studio's SDK Manager",
            check: ToolCheck::AndroidNdk,
        },
        Tool {
            name: "gradle",
            assembly: AssemblyKind::Android,
            check_display: "gradle --version",
            install: "brew install gradle",
            check: ToolCheck::Cmd {
                program: "gradle",
                args: &["--version"],
                min_version: None,
            },
        },
];

// ---------------------------------------------------------------------------
// Check execution
// ---------------------------------------------------------------------------

fn run_check(check: &ToolCheck) -> Result<(), String> {
    match check {
        ToolCheck::Cmd {
            program,
            args,
            min_version,
        } => run_cmd_check(program, args, *min_version),
        ToolCheck::Env { var, must_exist } => run_env_check(var, *must_exist),
        ToolCheck::RustupTargets(targets) => run_rustup_targets_check(targets),
        ToolCheck::AndroidNdk => run_android_ndk_check(),
    }
}

fn run_cmd_check(
    program: &str,
    args: &[&str],
    min_version: Option<Version>,
) -> Result<(), String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("failed to invoke {program}: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "{program} exited with status {:?}",
            output.status.code()
        ));
    }

    if let Some(min) = min_version {
        // Some tools (older javas, gradle) write the version banner to stderr
        // and stdout interchangeably. Search both.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}\n{stderr}");
        let found = extract_version(&combined)
            .ok_or_else(|| format!("could not parse version from {program} output"))?;
        if found < min {
            return Err(format!("found {found} but need >= {min}"));
        }
    }

    Ok(())
}

fn run_env_check(var: &str, must_exist: bool) -> Result<(), String> {
    let value = std::env::var(var).map_err(|_| format!("{var} not set"))?;
    if value.is_empty() {
        return Err(format!("{var} is empty"));
    }
    if must_exist && !PathBuf::from(&value).is_dir() {
        return Err(format!("{var}={value} does not point to an existing directory"));
    }
    Ok(())
}

fn run_rustup_targets_check(targets: &[&str]) -> Result<(), String> {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map_err(|e| format!("failed to invoke rustup: {e}"))?;
    if !output.status.success() {
        return Err("rustup target list --installed failed".into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let installed: HashSet<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let missing: Vec<&str> = targets
        .iter()
        .copied()
        .filter(|t| !installed.contains(t))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("missing rustup targets: {}", missing.join(", ")))
    }
}

fn run_android_ndk_check() -> Result<(), String> {
    let home = std::env::var("ANDROID_HOME")
        .map_err(|_| "ANDROID_HOME not set".to_string())?;
    let ndk = PathBuf::from(home).join("ndk");
    if !ndk.is_dir() {
        return Err(format!("{} not found", ndk.display()));
    }
    let any_entry = std::fs::read_dir(&ndk)
        .map_err(|e| format!("could not read {}: {e}", ndk.display()))?
        .next()
        .is_some();
    if !any_entry {
        return Err(format!("{} is empty", ndk.display()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Version parsing
// ---------------------------------------------------------------------------

/// A simple `major.minor.patch` triple. Sufficient for the floor checks the
/// CLI performs today (Java 21+); avoids pulling in the full `semver` crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse a strict `M.m[.p]` token. Returns `None` if the leading component
    /// isn't a number; trailing junk after the patch number (e.g.
    /// `1.8.0_221`'s `_221` suffix) is ignored.
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        let mut parts = trimmed.splitn(3, '.');
        let major: u32 = parts.next()?.parse().ok()?;
        let minor_raw = parts.next()?;
        let minor: u32 = leading_digits(minor_raw)?.parse().ok()?;
        let patch = match parts.next() {
            Some(p) => leading_digits(p)
                .map(|s| s.parse().unwrap_or(0))
                .unwrap_or(0),
            None => 0,
        };
        Some(Self::new(major, minor, patch))
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn leading_digits(s: &str) -> Option<&str> {
    let end = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    if end == 0 { None } else { Some(&s[..end]) }
}

/// Find the first `M.m[.p]` token anywhere in arbitrary tool output.
///
/// Splits on any non-`[0-9.]` character, then keeps tokens that contain a dot
/// (filters out year/build numbers like `2026`) and tries to parse each as a
/// [`Version`]. Returns the first successful parse.
pub(crate) fn extract_version(text: &str) -> Option<Version> {
    text.split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter(|s| !s.is_empty() && s.contains('.'))
        .find_map(|tok| Version::parse(tok.trim_matches('.')))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parse_basic() {
        assert_eq!(Version::parse("0.19.1"), Some(Version::new(0, 19, 1)));
        assert_eq!(Version::parse("21.0.10"), Some(Version::new(21, 0, 10)));
        assert_eq!(Version::parse("8.13"), Some(Version::new(8, 13, 0)));
    }

    #[test]
    fn version_parse_strips_suffix() {
        // Older Java patch suffix like `1.8.0_221` should ignore the suffix.
        assert_eq!(Version::parse("1.8.0_221"), Some(Version::new(1, 8, 0)));
    }

    #[test]
    fn version_parse_rejects_garbage() {
        assert_eq!(Version::parse("not.a.version"), None);
        assert_eq!(Version::parse(""), None);
        assert_eq!(Version::parse("1"), None);
    }

    #[test]
    fn version_ordering() {
        assert!(Version::new(21, 0, 0) > Version::new(17, 0, 5));
        assert!(Version::new(8, 13, 0) > Version::new(8, 12, 99));
        assert!(Version::new(0, 19, 1) >= Version::new(0, 19, 1));
    }

    #[test]
    fn version_display() {
        assert_eq!(format!("{}", Version::new(21, 0, 1)), "21.0.1");
    }

    #[test]
    fn extract_from_cargo_swift_output() {
        let out = "cargo-swift 0.9.0\n";
        assert_eq!(extract_version(out), Some(Version::new(0, 9, 0)));
    }

    #[test]
    fn extract_from_cargo_deny_output() {
        let out = "cargo-deny 0.19.1\n";
        assert_eq!(extract_version(out), Some(Version::new(0, 19, 1)));
    }

    #[test]
    fn extract_from_modern_java_output() {
        let out = "openjdk 21.0.10 2026-01-20 LTS\nOpenJDK Runtime Environment ...";
        assert_eq!(extract_version(out), Some(Version::new(21, 0, 10)));
    }

    #[test]
    fn extract_from_old_java_output() {
        // Old `java -version` emits `1.8.0_221` to stderr; we'd flag it as
        // below the 21.0.0 floor.
        let out = "java version \"1.8.0_221\"\nJava(TM) SE Runtime Environment ...";
        let found = extract_version(out).expect("should parse");
        assert_eq!(found, Version::new(1, 8, 0));
        assert!(found < Version::new(21, 0, 0));
    }

    #[test]
    fn extract_from_gradle_output() {
        let out = "------------------------------------------------------------\n\
                   Gradle 8.13\n\
                   ------------------------------------------------------------\n\
                   Build time: 2024-12-20 ...";
        assert_eq!(extract_version(out), Some(Version::new(8, 13, 0)));
    }

    #[test]
    fn extract_from_xcodegen_output() {
        // XcodeGen prints `Version: 2.42.0` (newline terminated).
        let out = "Version: 2.42.0\n";
        assert_eq!(extract_version(out), Some(Version::new(2, 42, 0)));
    }

    #[test]
    fn extract_skips_year_like_tokens() {
        // `2026` has no dot -> not a version. `2026-01-20` likewise.
        let out = "Released 2026-01-20, version 1.2.3";
        assert_eq!(extract_version(out), Some(Version::new(1, 2, 3)));
    }

    #[test]
    fn extract_returns_none_when_no_version() {
        assert_eq!(extract_version("no version here"), None);
        assert_eq!(extract_version(""), None);
    }

    #[test]
    fn assembly_tag_strings() {
        assert_eq!(AssemblyKind::Core.tag(), "core");
        assert_eq!(AssemblyKind::Ios.tag(), "ios");
        assert_eq!(AssemblyKind::Android.tag(), "android");
    }

    #[test]
    fn env_check_unset_var_is_failure() {
        // `VECTIS_TEST_DEFINITELY_NOT_SET` is reserved for tests.
        let err = run_env_check("VECTIS_TEST_DEFINITELY_NOT_SET", false).unwrap_err();
        assert!(err.contains("VECTIS_TEST_DEFINITELY_NOT_SET"));
    }

    #[test]
    fn env_check_empty_var_is_failure() {
        // SAFETY: tests in a binary crate run in the same process -- mutating
        // env vars is racy across threads but cargo test uses a single
        // thread per test by default for unit tests in this crate.
        // We use a reserved name to avoid colliding with anything real.
        unsafe {
            std::env::set_var("VECTIS_TEST_EMPTY", "");
        }
        let err = run_env_check("VECTIS_TEST_EMPTY", false).unwrap_err();
        assert!(err.contains("empty"));
        unsafe {
            std::env::remove_var("VECTIS_TEST_EMPTY");
        }
    }

    #[test]
    fn cmd_check_missing_program_fails() {
        let err = run_cmd_check("vectis-tool-that-does-not-exist", &["--version"], None)
            .unwrap_err();
        assert!(err.contains("vectis-tool-that-does-not-exist"));
    }

    #[test]
    fn cmd_check_min_version_too_low_fails() {
        // We can't easily mock Command output without an indirection layer;
        // the version comparison itself is exercised by the extract_* tests.
        // This test just confirms the wiring: a successful command with
        // unparseable output and a min_version returns an error.
        // `true` exits 0 with empty output -> "could not parse version".
        let err = run_cmd_check("true", &[], Some(Version::new(1, 0, 0))).unwrap_err();
        assert!(err.contains("could not parse version"));
    }
}
