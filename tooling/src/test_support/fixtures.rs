use std::fs;
use std::path::{Path, PathBuf};

use super::fixtures_dir;
use crate::Context;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFixture {
    pub name: String,
    pub dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetFixture {
    pub name: String,
    pub case_name: Option<String>,
    pub dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillFixture {
    pub skill: String,
    pub case_name: String,
    pub dir: PathBuf,
}

fn list_dirs(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|ft| ft.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

/// Every source fixture under `tests/fixtures/sources/<name>/`.
pub fn walk_source_fixtures(ctx: &Context) -> Vec<SourceFixture> {
    let root = fixtures_dir(ctx).join("sources");
    list_dirs(&root)
        .into_iter()
        .map(|name| {
            let dir = root.join(&name);
            SourceFixture { name, dir }
        })
        .collect()
}

/// Every target fixture under `tests/fixtures/targets/<name>/[<case>/]`.
pub fn walk_target_fixtures(ctx: &Context) -> Vec<TargetFixture> {
    let root = fixtures_dir(ctx).join("targets");
    let mut out = Vec::new();
    for name in list_dirs(&root) {
        let target_dir = root.join(&name);
        if target_dir.join("input").is_dir() {
            out.push(TargetFixture {
                name,
                case_name: None,
                dir: target_dir,
            });
            continue;
        }
        for case_name in list_dirs(&target_dir) {
            let case_dir = target_dir.join(&case_name);
            if case_dir.join("input").is_dir() {
                out.push(TargetFixture {
                    name: name.clone(),
                    case_name: Some(case_name),
                    dir: case_dir,
                });
            }
        }
    }
    out
}

/// Every skill fixture under `tests/fixtures/skills/<skill>/<case>/`.
pub fn walk_skill_fixtures(ctx: &Context, skill: &str) -> Vec<SkillFixture> {
    let root = fixtures_dir(ctx).join("skills").join(skill);
    list_dirs(&root)
        .into_iter()
        .map(|case_name| {
            let dir = root.join(&case_name);
            SkillFixture {
                skill: skill.to_string(),
                case_name,
                dir,
            }
        })
        .collect()
}
