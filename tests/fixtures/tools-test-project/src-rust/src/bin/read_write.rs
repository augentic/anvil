use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let project = PathBuf::from(env::var("PROJECT_DIR").expect("PROJECT_DIR is set"));
    let input = fs::read_to_string(project.join("inputs").join("probe.txt"))
        .expect("read inputs/probe.txt");
    let output = format!("derived: {}", input.trim());
    fs::write(project.join("outputs").join("result.txt"), &output).expect("write outputs/result.txt");
    println!("read-write: {output}");
}
