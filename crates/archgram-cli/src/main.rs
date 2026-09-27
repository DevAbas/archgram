//! The `archgram` command. Reads files, prints, sets the exit code; the
//! engine in `archgram-core` does everything else.
//!
//! Exit codes: 0 success, 1 the spec has problems, 2 the command itself is
//! wrong or a file cannot be read or written.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use archgram_core::SpecError;
use archgram_core::render::{Mode, Options};
use archgram_icons::Icons;

const USAGE: &str = "\
archgram: architecture diagrams from a spec

Usage:
  archgram build <spec> [-o <out.svg>] [--theme auto|light|dark | --split-themes] [--system-font]
                               Draw the diagram (default output: the spec's name with .svg)
  archgram check <spec>        Check a spec and list every problem
  archgram --version           Print the version
  archgram --help              Print this help

A spec is JSON (.json) or YAML (.yaml, .yml); YAML problems are shown at
their line and column.

Themes: auto (light, dark under the reader's dark mode; the default), light, dark.
--split-themes writes <out>.light.svg and <out>.dark.svg from one layout, for a
  page that picks one per reader (GitHub's <picture> with prefers-color-scheme).
--system-font leaves the text to the reader's font instead of embedding Geist.
  The text is still measured with Geist, so a wider system font can crowd or
  overflow a card; embedding (the default) draws exactly what was measured.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["check", path] => check(path),
        ["build", path, rest @ ..] => match build_options(rest) {
            Ok(b) => build(path, b),
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

/// What `build` was asked for.
struct Build {
    out: Option<PathBuf>,
    options: Options,
    /// Write one file per theme instead of one with both.
    split: bool,
}

fn build_options(rest: &[&str]) -> Result<Build, String> {
    let mut out = None;
    let mut options = Options::default();
    let (mut split, mut themed) = (false, false);
    let mut it = rest.iter();
    while let Some(arg) = it.next() {
        match *arg {
            "-o" | "--output" => out = Some(PathBuf::from(it.next().ok_or("-o needs a file")?)),
            "--theme" => {
                themed = true;
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
            "--split-themes" => split = true,
            other => return Err(format!("unrecognised option {other}")),
        }
    }
    if split && themed {
        return Err("--split-themes writes both themes; leave out --theme".into());
    }
    Ok(Build { out, options, split })
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

/// The formats a spec may be written in, by its file's extension.
#[derive(Clone, Copy)]
enum Format {
    Json,
    Yaml,
}

fn format_of(path: &str) -> Result<Format, ExitCode> {
    match Path::new(path).extension().and_then(|e| e.to_str()) {
        Some("json") => Ok(Format::Json),
        Some("yaml" | "yml") => Ok(Format::Yaml),
        _ => Err(usage_error(&format!(
            "{path}: a spec ends in .json, .yaml or .yml"
        ))),
    }
}

/// Reads and checks a spec in its format, each `tech` against the logos
/// archgram carries; a YAML spec's problems are at its lines and columns.
fn parse(text: &str, format: Format) -> Result<archgram_core::Spec, Vec<SpecError>> {
    let icons = Icons::load();
    match format {
        Format::Json => {
            let spec = archgram_core::parse_spec(text)?;
            let errors = archgram_core::check_logos(&spec, &icons);
            if errors.is_empty() { Ok(spec) } else { Err(errors) }
        }
        Format::Yaml => {
            let (spec, positions) = archgram_yaml::parse(text)?;
            let errors: Vec<SpecError> = archgram_core::check_logos(&spec, &icons)
                .into_iter()
                .map(|e| positions.locate(e))
                .collect();
            if errors.is_empty() { Ok(spec) } else { Err(errors) }
        }
    }
}

fn check(path: &str) -> ExitCode {
    let format = match format_of(path) {
        Ok(f) => f,
        Err(code) => return code,
    };
    let text = match read(path) {
        Ok(t) => t,
        Err(code) => return code,
    };
    match parse(&text, format) {
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

fn build(path: &str, Build { out, options, split }: Build) -> ExitCode {
    let format = match format_of(path) {
        Ok(f) => f,
        Err(code) => return code,
    };
    let text = match read(path) {
        Ok(t) => t,
        Err(code) => return code,
    };
    let spec = match parse(&text, format) {
        Ok(spec) => spec,
        Err(errors) => return report(path, &errors),
    };
    if options.embed_font {
        let missing = archgram_core::uncovered_characters(&spec, &Icons::load());
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
    let out = out.unwrap_or_else(|| Path::new(path).with_extension("svg"));
    let files = if split {
        match archgram_core::draw_themes(&spec, options, &Icons::load()) {
            Ok((light, dark)) => vec![(themed(&out, "light"), light), (themed(&out, "dark"), dark)],
            Err(errors) => return report(path, &errors),
        }
    } else {
        match archgram_core::draw_with(&spec, options, &Icons::load()) {
            Ok(svg) => vec![(out, svg)],
            Err(errors) => return report(path, &errors),
        }
    };
    for (file, svg) in files {
        if let Err(e) = std::fs::write(&file, svg) {
            eprintln!("archgram: cannot write {}: {e}", file.display());
            return ExitCode::from(2);
        }
        println!("wrote {}", file.display());
    }
    ExitCode::SUCCESS
}

/// `diagram.svg` as `diagram.light.svg`, for one theme's file.
fn themed(out: &Path, theme: &str) -> PathBuf {
    out.with_extension(format!("{theme}.svg"))
}
