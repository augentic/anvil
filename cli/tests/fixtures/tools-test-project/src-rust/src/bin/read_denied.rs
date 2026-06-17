use std::fs;

fn main() {
    match fs::read_to_string("/etc/passwd") {
        Ok(text) => {
            println!("unexpected-read: {}", text.len());
        }
        Err(err) => {
            eprintln!("read-denied: {err}");
            std::process::exit(13);
        }
    }
}
