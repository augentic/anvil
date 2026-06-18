use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let project = PathBuf::from(env::var("PROJECT_DIR").expect("PROJECT_DIR is set"));
    let text = fs::read_to_string(project.join("inputs").join("probe.txt"))
        .expect("read inputs/probe.txt");
    println!("read-only: {}", text.trim());
}
