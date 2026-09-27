//! The `archgram` command. Reads files, prints, sets the exit code; the
//! engine in `archgram-core` does everything else.
//!
//! Exit codes: 0 success, 1 the spec has problems, 2 the command itself is
//! wrong or a file cannot be read.

use std::process::ExitCode;

const USAGE: &str = "\
archgram: architecture diagrams from a spec

Usage:
  archgram check <spec.json>   Check a spec and list every problem
  archgram --version           Print the version
  archgram --help              Print this help";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["check", path] => check(path),
        ["--version" | "-V"] => {
            println!("archgram {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        ["--help" | "-h"] | [] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("archgram: unrecognised arguments: {}\n\n{USAGE}", args.join(" "));
            ExitCode::from(2)
        }
    }
}

fn check(path: &str) -> ExitCode {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("archgram: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    match archgram_core::parse_spec(&text) {
        Ok(spec) => {
            println!(
                "{path}: valid ({} nodes, {} edges)",
                spec.nodes.len(),
                spec.edges.len()
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{path}:{e}");
            }
            eprintln!(
                "{} problem{} in {path}",
                errors.len(),
                if errors.len() == 1 { "" } else { "s" }
            );
            ExitCode::from(1)
        }
    }
}
