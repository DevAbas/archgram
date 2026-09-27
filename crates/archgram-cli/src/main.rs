//! The `archgram` command. Reads files, prints, sets the exit code; the
//! engine in `archgram-core` does everything else.
//!
//! Exit codes: 0 success, 1 the spec has problems, 2 the command itself is
//! wrong or a file cannot be read or written.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use archgram_core::SpecError;
use archgram_core::render::{Mode, Options};

const USAGE: &str = "\
archgram: architecture diagrams from a spec

Usage:
  archgram build <spec.json> [-o <out.svg>] [--theme auto|light|dark] [--system-font]
                               Draw the diagram (default output: the spec's name with .svg)
  archgram check <spec.json>   Check a spec and list every problem
  archgram --version           Print the version
  archgram --help              Print this help

Themes: auto (light, dark under the reader's dark mode; the default), light, dark.
--system-font leaves the text to the reader's font instead of embedding Geist.
  The text is still measured with Geist, so a wider system font can crowd or
  overflow a card; embedding (the default) draws exactly what was measured.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["check", path] => check(path),
        ["build", path, rest @ ..] => match build_options(rest) {
            Ok((out, options)) => build(path, out, options),
            Err(message) => usage_error(&message),
        },
        ["--version" | "-V"] => {
            println!("archgram {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        ["--help" | "-h"] | [] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => usage_error(&format!("unrecognised arguments: {}", args.join(" "))),
    }
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("archgram: {message}\n\n{USAGE}");
    ExitCode::from(2)
}

fn build_options(rest: &[&str]) -> Result<(Option<PathBuf>, Options), String> {
    let mut out = None;
    let mut options = Options::default();
    let mut it = rest.iter();
    while let Some(arg) = it.next() {
        match *arg {
            "-o" | "--output" => out = Some(PathBuf::from(it.next().ok_or("-o needs a file")?)),
            "--theme" => {
                options.mode = match it.next().copied() {
                    Some("auto") => Mode::Auto,
                    Some("light") => Mode::Light,
                    Some("dark") => Mode::Dark,
                    other => {
                        return Err(format!(
                            "--theme takes auto, light or dark, not {}",
                            other.unwrap_or("nothing")
                        ));
                    }
                };
            }
            "--system-font" => options.embed_font = false,
            other => return Err(format!("unrecognised option {other}")),
        }
    }
    Ok((out, options))
}

fn read(path: &str) -> Result<String, ExitCode> {
    std::fs::read_to_string(path).map_err(|e| {
        eprintln!("archgram: cannot read {path}: {e}");
        ExitCode::from(2)
    })
}

fn report(path: &str, errors: &[SpecError]) -> ExitCode {
    for e in errors {
        eprintln!("{path}:{e}");
    }
    eprintln!(
        "{} problem{} in {path}",
        errors.len(),
        if errors.len() == 1 { "" } else { "s" }
    );
    ExitCode::from(1)
}

fn check(path: &str) -> ExitCode {
    let text = match read(path) {
        Ok(t) => t,
        Err(code) => return code,
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
        Err(errors) => report(path, &errors),
    }
}

fn build(path: &str, out: Option<PathBuf>, options: Options) -> ExitCode {
    let text = match read(path) {
        Ok(t) => t,
        Err(code) => return code,
    };
    let spec = match archgram_core::parse_spec(&text) {
        Ok(spec) => spec,
        Err(errors) => return report(path, &errors),
    };
    if options.embed_font {
        let missing = archgram_core::uncovered_characters(&spec);
        if !missing.is_empty() {
            let list = missing
                .iter()
                .map(|c| format!("{c} (U+{:04X})", u32::from(*c)))
                .collect::<Vec<_>>()
                .join(" ");
            eprintln!(
                "archgram: warning: the embedded font lacks {}; the reader's font will draw {}: {}",
                if missing.len() == 1 {
                    "a character"
                } else {
                    "some characters"
                },
                if missing.len() == 1 { "it" } else { "them" },
                list
            );
        }
    }
    let svg = match archgram_core::draw(&spec, options) {
        Ok(svg) => svg,
        Err(errors) => return report(path, &errors),
    };
    let out = out.unwrap_or_else(|| Path::new(path).with_extension("svg"));
    if let Err(e) = std::fs::write(&out, svg) {
        eprintln!("archgram: cannot write {}: {e}", out.display());
        return ExitCode::from(2);
    }
    println!("wrote {}", out.display());
    ExitCode::SUCCESS
}
