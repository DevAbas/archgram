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
                     [--theme-file <archgram.theme.json>]
                               Draw the diagram (default output: the spec's name with .svg)
  archgram check <spec>        Check a spec and list every problem
  archgram spec                Print the spec format this archgram reads (docs/SPEC.md)
  archgram theme check <archgram.theme.json>
                               Read a project's design tokens as the theme and show each role's colour
  archgram --version           Print the version
  archgram --help              Print this help

A spec is JSON (.json) or YAML (.yaml, .yml); YAML problems are shown at
their line and column. A spec named <name>.archgram.yaml (or .yml, .json)
draws <name>.svg beside it. -o names another file, and creates its folder
when it does not exist.

Themes: auto (light, dark under the reader's dark mode; the default), light, dark.
--split-themes writes <out>.light.svg and <out>.dark.svg from one layout, for a
  page that picks one per reader (GitHub's <picture> with prefers-color-scheme).
--system-font leaves the text to the reader's font instead of embedding Geist.
  The text is still measured with Geist, so a wider system font can crowd or
  overflow a card; embedding (the default) draws exactly what was measured.
--theme-file draws in a project's own colours: a mapping file names the
  project's DTCG resolver, its light and dark inputs, and the token for each
  role (docs/SPEC.md, Theme file).";

/// The spec format this archgram reads, carried in the binary so a spec's
/// writer, a person or an agent, reads the format of the very command that
/// draws it (docs/PRD.md, 6.5).
const SPEC: &str = include_str!("../../../docs/SPEC.md");

fn main() -> ExitCode {
    // A panic is archgram's bug, never the spec's: say so, and where to
    // report it, instead of a bare Rust panic.
    std::panic::set_hook(Box::new(|info| {
        eprintln!(
            "archgram {}: an internal error, a bug in archgram: {info}\n\
             Please report it, with the spec, at https://github.com/DevAbas/archgram/issues",
            env!("CARGO_PKG_VERSION")
        );
    }));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["check", path] => check(path),
        ["spec"] => {
            print!("{SPEC}");
            ExitCode::SUCCESS
        }
        ["theme", "check", path] => theme_check(path),
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
    /// A mapping file for a project's own colours.
    theme_file: Option<String>,
}

fn build_options(rest: &[&str]) -> Result<Build, String> {
    let mut out = None;
    let mut options = Options::default();
    let (mut split, mut themed) = (false, false);
    let mut theme_file = None;
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
            "--theme-file" => {
                theme_file = Some((*it.next().ok_or("--theme-file needs a file")?).to_owned());
            }
            other => return Err(format!("unrecognised option {other}")),
        }
    }
    if split && themed {
        return Err("--split-themes writes both themes; leave out --theme".into());
    }
    Ok(Build {
        out,
        options,
        split,
        theme_file,
    })
}

fn read(path: &str) -> Result<String, ExitCode> {
    std::fs::read_to_string(path).map_err(|e| {
        eprintln!("archgram: cannot read {path}: {e}");
        ExitCode::from(2)
    })
}

fn report(path: &str, errors: &[SpecError]) -> ExitCode {
    for e in errors {
        eprintln!("{path}:{}", printable(&e.to_string()));
    }
    eprintln!(
        "{} problem{} in {path}",
        errors.len(),
        if errors.len() == 1 { "" } else { "s" }
    );
    ExitCode::from(1)
}

/// `text` with each control character but the line feed written as its
/// escape (`\u{1b}`), so a spec or theme printed back cannot drive the
/// terminal or a CI log.
fn printable(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_control() && c != '\n' {
                c.escape_debug().to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
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
            if errors.is_empty() {
                Ok(spec)
            } else {
                Err(errors)
            }
        }
        Format::Yaml => {
            let (spec, positions) = archgram_yaml::parse(text)?;
            let errors: Vec<SpecError> = archgram_core::check_logos(&spec, &icons)
                .into_iter()
                .map(|e| positions.locate(e))
                .collect();
            if errors.is_empty() {
                Ok(spec)
            } else {
                Err(errors)
            }
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

fn build(
    path: &str,
    Build {
        out,
        mut options,
        split,
        theme_file,
    }: Build,
) -> ExitCode {
    if let Some(file) = theme_file {
        match load_theme(&file) {
            Ok(imported) => options.colors = Some(imported.colors),
            Err(code) => return code,
        }
    }
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
    let out = out.unwrap_or_else(|| default_output(path));
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
        if let Some(folder) = file.parent().filter(|f| !f.as_os_str().is_empty())
            && let Err(e) = std::fs::create_dir_all(folder)
        {
            eprintln!(
                "archgram: cannot create the folder {}: {e}",
                folder.display()
            );
            return ExitCode::from(2);
        }
        if let Err(e) = write_replacing(&file, &svg) {
            eprintln!("archgram: cannot write {}: {e}", file.display());
            return ExitCode::from(2);
        }
        println!("wrote {}", file.display());
    }
    ExitCode::SUCCESS
}

/// Writes `contents` to `path` as a new file beside it, renamed over the
/// old one, as Turborepo restores its cache: a symlink at `path` is
/// replaced, never followed, so a spec cannot aim the drawing at another
/// file, and a reader never sees half a drawing.
fn write_replacing(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a file name"))?;
    let temporary = path.with_file_name(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    // `create_new` refuses any file already there, a symlink too.
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .and_then(|mut file| file.write_all(contents.as_bytes()));
    let renamed = written.and_then(|()| std::fs::rename(&temporary, path));
    if renamed.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    renamed
}

/// Where a spec draws when `-o` names no file (docs/SPEC.md, Formats):
/// `harness.archgram.yaml` draws `harness.svg` beside it, so the drawing
/// keeps a plain name; any other spec draws its own name with `.svg`.
fn default_output(spec: &str) -> PathBuf {
    let path = Path::new(spec);
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    for suffix in [".archgram.yaml", ".archgram.yml", ".archgram.json"] {
        if let Some(stem) = name.strip_suffix(suffix)
            && !stem.is_empty()
        {
            return path.with_file_name(format!("{stem}.svg"));
        }
    }
    path.with_extension("svg")
}

/// `diagram.svg` as `diagram.light.svg`, for one theme's file.
fn themed(out: &Path, theme: &str) -> PathBuf {
    out.with_extension(format!("{theme}.svg"))
}

/// Reads a theme's mapping file and the tokens it names, each file by its
/// path beside the mapping and only under the mapping's folder; prints the
/// problems if there are any.
fn load_theme(path: &str) -> Result<archgram_core::theme::Imported, ExitCode> {
    read(path)?;
    // The core joins paths with `/` alone, so it is handed the mapping's
    // name and each file's path from there. Each file is found from the
    // folder as written, where Windows reads `/` and `..` too, and held to
    // the folder's real path, which on Windows is a `\\?\` path that
    // reads neither.
    let real = std::fs::canonicalize(path).map_err(|e| {
        eprintln!("archgram: cannot read {path}: {e}");
        ExitCode::from(2)
    })?;
    let written = Path::new(path).parent().unwrap_or(Path::new(""));
    let (Some(folder), Some(name)) = (real.parent(), real.file_name().and_then(|n| n.to_str()))
    else {
        eprintln!("archgram: {path} is not a file in a folder");
        return Err(ExitCode::from(2));
    };
    let from_disk = |p: &str| read_under(folder, &written.join(p));
    archgram_core::theme::import(name, &from_disk).map_err(|errors| {
        for e in &errors {
            eprintln!("{}", printable(&e.to_string()));
        }
        eprintln!(
            "{} problem{} in the theme {path}",
            errors.len(),
            if errors.len() == 1 { "" } else { "s" }
        );
        ExitCode::from(1)
    })
}

/// The text of the file at `path` when its real path is under `folder`, as
/// Turborepo keeps a workspace's files within its repository: neither `..`
/// nor an absolute path nor a symlink leads a theme out of its project, and
/// only a regular file is read, never a device.
fn read_under(folder: &Path, path: &Path) -> Result<String, String> {
    let cannot = |_| "the file cannot be read".to_owned();
    let real = std::fs::canonicalize(path).map_err(cannot)?;
    if !real.starts_with(folder) {
        // Not the folder's real path: on Windows it is a `\\?\` path, which
        // std::fs::canonicalize warns other programs may not read.
        return Err(
            "the file is outside the mapping file's folder, and archgram reads only the files under it"
                .into(),
        );
    }
    if !real.is_file() {
        return Err("not a file".into());
    }
    std::fs::read_to_string(&real).map_err(cannot)
}

fn theme_check(path: &str) -> ExitCode {
    let imported = match load_theme(path) {
        Ok(i) => i,
        Err(code) => return code,
    };
    for (name, roles) in [("light", &imported.light), ("dark", &imported.dark)] {
        println!("{name}:");
        for (role, source, colour) in roles {
            let from = source
                .as_deref()
                .map_or_else(|| "(mono)".to_owned(), |id| format!("{{{id}}}"));
            println!("  {:<12} {colour}  {}", role.name(), printable(&from));
        }
    }
    println!("{path}: every pair keeps its contrast in both themes");
    ExitCode::SUCCESS
}
