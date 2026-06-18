use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    println!("echo: {}", args.join(" "));
    println!("PROJECT_DIR={}", env::var("PROJECT_DIR").unwrap_or_else(|_| "<unset>".to_string()));
    println!(
        "CAPABILITY_DIR={}",
        env::var("CAPABILITY_DIR").unwrap_or_else(|_| "<unset>".to_string())
    );
    println!("PATH={}", env::var("PATH").unwrap_or_else(|_| "<unset>".to_string()));
}
