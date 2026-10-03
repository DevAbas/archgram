//! Repository tasks, run with `cargo xtask <task>` (ARCHITECTURE.md, Code map).
//!
//! `icons <tag> <commit>` writes `crates/archgram-icons/data` from a pinned
//! release of Simple Icons (ARCHITECTURE.md, Code map): a shallow clone of
//! the tag under `target/`, refused unless the tag is at `commit`, since a
//! tag can be moved and the commit cannot; its `data/simple-icons.json` for titles and licences,
//! `slugs.md` for each title's slug, and `icons/<slug>.svg` for its path.
//! An icon that carries a licence of its own other than CC0 is left out.
//!
//! `fonts` writes the fonts archgram embeds (`crates/archgram-core/fonts/`)
//! from Geist's release files in `fonts/source/`: every character each one
//! maps, through the core's own subsetter, which keeps only the tables a
//! renderer needs. Kerning, ligatures, glyph names and hinting go; no
//! glyph's outline or advance changes.
//!
//! `npm` writes the npm packages from the built binaries (`npm.rs`).

mod npm;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::Value;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["icons", tag, commit] => icons(tag, commit),
        ["fonts"] => fonts(),
        ["npm", flags @ ..] => npm::npm(flags),
        _ => {
            eprintln!(
                "usage: cargo xtask icons <simple-icons tag> <its commit> | cargo xtask fonts | cargo xtask npm [--all] [--smoke]"
            );
            ExitCode::from(2)
        }
    }
}

fn fonts() -> ExitCode {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/archgram-core/fonts");
    for name in ["Geist-Regular", "Geist-Medium"] {
        let source = dir.join(format!("source/{name}.ttf"));
        let written = std::fs::read(&source)
            .map_err(|e| format!("{}: {e}", source.display()))
            .and_then(|font| {
                let chars = archgram_core::font::characters(&font)?;
                let out = archgram_core::font::subset::subset(&font, &chars)
                    .map_err(|e| format!("{e:?}"))?;
                Ok((font.len(), chars.len(), out))
            });
        match written {
            Ok((before, chars, out)) => {
                let target = dir.join(format!("{name}.ttf"));
                if let Err(e) = std::fs::write(&target, &out) {
                    eprintln!("xtask: {}: {e}", target.display());
                    return ExitCode::FAILURE;
                }
                println!(
                    "{name}: {chars} characters, {before} bytes to {}",
                    out.len()
                );
            }
            Err(e) => {
                eprintln!("xtask: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

const SIMPLE_ICONS: &str = "https://github.com/simple-icons/simple-icons";

fn icons(tag: &str, commit: &str) -> ExitCode {
    match write_icons(tag, commit) {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("icons: {e}");
            ExitCode::FAILURE
        }
    }
}

/// A shallow clone of Simple Icons' `tag` under `target/`, and its commit,
/// refused unless the tag is at `pinned`: a tag can be moved to another
/// commit, the commit named with it cannot.
fn pinned_clone(tag: &str, pinned: &str) -> Result<(PathBuf, String), String> {
    if pinned.len() != 40 || !pinned.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!(
            "`{pinned}` is not a full commit hash (40 hex digits)"
        ));
    }
    let dir = workspace_root()
        .join("target")
        .join(format!("simple-icons-{tag}"));
    if !dir.join(".git").exists() {
        let status = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--quiet",
                "--branch",
                tag,
                SIMPLE_ICONS,
            ])
            .arg(&dir)
            .status()
            .map_err(|e| format!("cannot run git: {e}"))?;
        if !status.success() {
            return Err(format!("git clone of {tag} failed: {status}"));
        }
    }
    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&dir)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    let commit = String::from_utf8_lossy(&commit.stdout).trim().to_owned();
    if !commit.eq_ignore_ascii_case(pinned) {
        return Err(format!(
            "simple-icons {tag} is at {commit}, not {pinned}: the tag has moved, or the commit is wrong; check the release before pinning another commit"
        ));
    }
    Ok((dir, commit))
}

fn write_icons(tag: &str, pinned: &str) -> Result<String, String> {
    let root = workspace_root();
    let (dir, commit) = pinned_clone(tag, pinned)?;
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).map_err(|e| format!("{p}: {e}"));

    let slug_of = slug_table(&read("slugs.md")?);
    let data: Value =
        serde_json::from_str(&read("data/simple-icons.json")?).map_err(|e| e.to_string())?;
    let entries = data.as_array().ok_or("simple-icons.json is not an array")?;
    // Slugs the data names outright; a title shared by two icons leaves the
    // other one to the icon without a slug of its own.
    let claimed: std::collections::BTreeSet<&str> =
        entries.iter().filter_map(|e| e["slug"].as_str()).collect();
    let mut rows = Vec::new();
    let mut provenance = Vec::new();
    let mut left_out = 0;
    for entry in entries {
        let title = entry["title"].as_str().ok_or("an icon without a title")?;
        let licence = entry["license"]["type"].as_str();
        if licence.is_some_and(|l| l != "CC0-1.0") {
            left_out += 1;
            continue;
        }
        let slug = if let Some(s) = entry["slug"].as_str() {
            s.to_owned()
        } else {
            slug_for(title, &slug_of, &claimed, &dir)?
        };
        let svg = read(&format!("icons/{slug}.svg"))?;
        let path = svg
            .split_once(" d=\"")
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(d, _)| d)
            .ok_or(format!("icons/{slug}.svg has no path"))?;
        if title.contains('\t') || path.contains('\t') {
            return Err(format!("`{slug}` holds a tab"));
        }
        let hex = entry["hex"]
            .as_str()
            .ok_or(format!("`{title}` has no colour"))?;
        rows.push(format!("{slug}\t{title}\t{hex}\t{path}"));
        provenance.push(format!(
            "{slug}\t{}\t{}\t{}",
            entry["source"].as_str().unwrap_or(""),
            entry["guidelines"].as_str().unwrap_or(""),
            licence.unwrap_or("")
        ));
    }
    rows.sort();
    provenance.sort();
    if rows
        .windows(2)
        .any(|w| w[0].split('\t').next() == w[1].split('\t').next())
    {
        return Err("two icons share a slug".into());
    }
    let out = root.join("crates/archgram-icons/data");
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let write = |name: &str, text: String| {
        std::fs::write(out.join(name), text).map_err(|e| format!("{name}: {e}"))
    };
    write("icons.tsv", rows.join("\n") + "\n")?;
    write("provenance.tsv", provenance.join("\n") + "\n")?;
    write(
        "RELEASE",
        format!(
            "simple-icons {tag}\ncommit {commit}\nicons {}\nleft out (a licence other than CC0) {left_out}\n",
            rows.len()
        ),
    )?;
    Ok(format!(
        "icons: {} from simple-icons {tag} ({commit}); {left_out} left out for a licence other than CC0",
        rows.len()
    ))
}

/// Title to slugs, from the table Simple Icons publishes with a release.
fn slug_table(md: &str) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut slug_of: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for line in md.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let unquote = |c: &str| {
            c.strip_prefix('`')
                .and_then(|c| c.strip_suffix('`'))
                .map(str::to_owned)
        };
        if let [_, title, slug, _] = cells.as_slice()
            && let (Some(t), Some(s)) = (unquote(title), unquote(slug))
        {
            slug_of.entry(t).or_default().push(s);
        }
    }
    slug_of
}

/// An icon's slug from its title: the table's, less slugs other icons
/// claim; for a title the table lacks, Simple Icons' rule for a plain ASCII
/// title (lower case, `+` plus, `.` dot, `&` and, nothing else but letters
/// and digits), taken only when that icon's file exists.
fn slug_for(
    title: &str,
    slug_of: &std::collections::BTreeMap<String, Vec<String>>,
    claimed: &std::collections::BTreeSet<&str>,
    dir: &Path,
) -> Result<String, String> {
    let free: Vec<&String> = slug_of
        .get(title)
        .map(|all| {
            all.iter()
                .filter(|s| !claimed.contains(s.as_str()))
                .collect()
        })
        .unwrap_or_default();
    match free.as_slice() {
        [one] => Ok((*one).clone()),
        [] if title.is_ascii() => {
            let slug: String = title
                .to_lowercase()
                .replace('+', "plus")
                .replace('.', "dot")
                .replace('&', "and")
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .collect();
            if dir.join(format!("icons/{slug}.svg")).exists() {
                Ok(slug)
            } else {
                Err(format!(
                    "`{title}` is not in slugs.md, and icons/{slug}.svg does not exist"
                ))
            }
        }
        [] => Err(format!("`{title}` is not in slugs.md")),
        many => Err(format!("`{title}` has several slugs in slugs.md: {many:?}")),
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace")
        .to_path_buf()
}
