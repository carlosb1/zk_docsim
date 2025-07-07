use std::process::Command;
use std::fs;

fn main() {
    let a = fs::read_to_string("embeddings/doc1.json").unwrap();
    let b = fs::read_to_string("embeddings/doc2.json").unwrap();
    // guardas en Noir input.toml y ejecutas:
    // nargo execute
    let _ = Command::new("nargo")
        .arg("execute")
        .status()
        .expect("failed to run noir");
}
