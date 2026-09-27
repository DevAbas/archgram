//! Repository tasks, run with `cargo xtask <task>` (ARCHITECTURE.md, Code map).
//!
//! `deps` checks every package in the dependency tree without third-party
//! tools (ARCHITECTURE.md, Dependencies and supply chain):
//!
//! - its licence, from `cargo metadata`, against the allowed list;
//! - its version, from `Cargo.lock`, against the RustSec advisory database,
//!   which is a git repository of one `Markdown` file per advisory.
//!
//! A licence outside the list or a known vulnerability fails the task. An
//! informational advisory (unmaintained, unsound) is reported but does not.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::Value;

/// Licences a dependency may carry: permissive, compatible with archgram's MIT
/// licence. OFL-1.1 covers the embedded font.
const ALLOWED_LICENCES: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Zlib",
    "OFL-1.1",
];

const ADVISORY_DB: &str = "https://github.com/rustsec/advisory-db";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some("deps") = args.first().map(String::as_str) {
        deps()
    } else {
        eprintln!("usage: cargo xtask deps");
        ExitCode::from(2)
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace")
        .to_path_buf()
}

fn deps() -> ExitCode {
    let root = workspace_root();
    let packages = match metadata(&root) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("deps: {e}");
            return ExitCode::from(2);
        }
    };
    let mut failed = false;

    // Licences.
    let mut bad = Vec::new();
    for p in &packages {
        match &p.licence {
            Some(expr) if licence_allowed(expr) => {}
            Some(expr) => bad.push(format!(
                "{} {}: licence `{expr}` is not in the allowed list",
                p.name, p.version
            )),
            None => bad.push(format!("{} {}: no licence declared", p.name, p.version)),
        }
    }
    if bad.is_empty() {
        println!("licences: {} packages, all allowed", packages.len());
    } else {
        failed = true;
        for b in &bad {
            eprintln!("licence: {b}");
        }
    }

    // Sources: everything outside the workspace comes from crates.io, so every
    // dependency has a place in the advisory database and a published licence.
    let foreign: Vec<&Package> = packages
        .iter()
        .filter(|p| !p.in_workspace && p.source.as_deref() != Some(CRATES_IO))
        .collect();
    if foreign.is_empty() {
        println!("sources: every dependency comes from crates.io");
    } else {
        failed = true;
        for p in &foreign {
            eprintln!(
                "source: {} {} comes from {}, not crates.io",
                p.name,
                p.version,
                p.source.as_deref().unwrap_or("a local path")
            );
        }
    }

    // Advisories, for the crates.io packages (our own crates have none).
    let db = root.join("target").join("advisory-db");
    if let Err(e) = fetch_advisory_db(&db) {
        eprintln!("advisories: cannot fetch the RustSec database: {e}");
        return ExitCode::from(2);
    }
    let external: Vec<&Package> = packages.iter().filter(|p| !p.in_workspace).collect();
    let mut hits = 0;
    for p in &external {
        for a in advisories_for(&db, &p.name) {
            if a.withdrawn || !a.affects(&p.version) {
                continue;
            }
            if a.informational.is_some() {
                println!(
                    "advisory (informational, {}): {} {} {}",
                    a.informational.as_deref().unwrap_or_default(),
                    p.name,
                    p.version,
                    a.id
                );
            } else {
                failed = true;
                hits += 1;
                eprintln!(
                    "advisory: {} {} is affected by {} ({})",
                    p.name, p.version, a.id, a.title
                );
            }
        }
    }
    if hits == 0 {
        println!(
            "advisories: {} registry packages checked, none affected",
            external.len()
        );
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[derive(Debug)]
struct Package {
    name: String,
    version: Version,
    licence: Option<String>,
    /// `cargo metadata`'s source: crates.io, another registry, git, or none for a path.
    source: Option<String>,
    /// One of this workspace's own crates.
    in_workspace: bool,
}

/// The source `cargo metadata` gives crates.io packages, whichever protocol fetched them.
const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

/// Every package Cargo resolves for the workspace, from `cargo metadata --locked`.
fn metadata(root: &Path) -> Result<Vec<Package>, String> {
    let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["metadata", "--format-version", "1", "--locked"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    let json: Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    let mut packages = Vec::new();
    let members: Vec<&str> = json["workspace_members"]
        .as_array()
        .ok_or("cargo metadata has no workspace members")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for p in json["packages"]
        .as_array()
        .ok_or("cargo metadata has no packages")?
    {
        let name = p["name"].as_str().unwrap_or_default().to_owned();
        let version = Version::parse(p["version"].as_str().unwrap_or_default())
            .ok_or_else(|| format!("{name}: unreadable version"))?;
        packages.push(Package {
            name,
            version,
            licence: p["license"].as_str().map(str::to_owned),
            source: p["source"].as_str().map(str::to_owned),
            in_workspace: p["id"].as_str().is_some_and(|id| members.contains(&id)),
        });
    }
    packages.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    Ok(packages)
}

/// An SPDX expression is allowed when it can be satisfied with allowed licences:
/// `A OR B` needs one side, `A AND B` needs both, parentheses group.
fn licence_allowed(expr: &str) -> bool {
    let tokens: Vec<String> = expr
        .replace('(', " ( ")
        .replace(')', " ) ")
        .replace('/', " OR ")
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let mut pos = 0;
    let ok = parse_or(&tokens, &mut pos);
    ok && pos == tokens.len()
}

fn parse_or(t: &[String], pos: &mut usize) -> bool {
    let mut ok = parse_and(t, pos);
    while t.get(*pos).is_some_and(|x| x == "OR") {
        *pos += 1;
        ok |= parse_and(t, pos);
    }
    ok
}

fn parse_and(t: &[String], pos: &mut usize) -> bool {
    let mut ok = parse_term(t, pos);
    while t.get(*pos).is_some_and(|x| x == "AND") {
        *pos += 1;
        ok &= parse_term(t, pos);
    }
    ok
}

fn parse_term(t: &[String], pos: &mut usize) -> bool {
    match t.get(*pos).map(String::as_str) {
        Some("(") => {
            *pos += 1;
            let ok = parse_or(t, pos);
            if t.get(*pos).is_some_and(|x| x == ")") {
                *pos += 1;
            }
            ok
        }
        Some(id) => {
            *pos += 1;
            let mut licence = id.to_owned();
            if t.get(*pos).is_some_and(|x| x == "WITH") {
                licence = format!("{licence} WITH {}", t.get(*pos + 1).map_or("", String::as_str));
                *pos += 2;
            }
            ALLOWED_LICENCES.contains(&licence.as_str())
        }
        None => false,
    }
}

/// Clones the advisory database on first use, then fast-forwards it.
fn fetch_advisory_db(dir: &Path) -> Result<(), String> {
    let status = if dir.join(".git").exists() {
        Command::new("git")
            .args(["-C"])
            .arg(dir)
            .args(["pull", "--ff-only", "--quiet"])
            .status()
    } else {
        Command::new("git")
            .args(["clone", "--depth", "1", "--quiet", ADVISORY_DB])
            .arg(dir)
            .status()
    };
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("git exited with {s}")),
        Err(e) => Err(format!("cannot run git: {e}")),
    }
}

#[derive(Debug, Default)]
struct Advisory {
    id: String,
    title: String,
    informational: Option<String>,
    withdrawn: bool,
    patched: Vec<Requirement>,
    unaffected: Vec<Requirement>,
}

impl Advisory {
    /// A version is affected unless a patched or unaffected range covers it.
    fn affects(&self, v: &Version) -> bool {
        !self.patched.iter().chain(&self.unaffected).any(|r| r.matches(v))
    }
}

/// The advisories filed against one crate: `crates/<name>/*.md` in the database.
fn advisories_for(db: &Path, name: &str) -> Vec<Advisory> {
    let Ok(entries) = std::fs::read_dir(db.join("crates").join(name)) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    files.sort();
    files
        .iter()
        .filter_map(|f| std::fs::read_to_string(f).ok())
        .map(|text| parse_advisory(&text))
        .collect()
}

/// Reads the TOML front matter of an advisory (between `` ```toml `` fences) and its title
/// (the first `# ` heading). Only the fields the check needs are read.
fn parse_advisory(text: &str) -> Advisory {
    let mut a = Advisory::default();
    let toml = text
        .split("```toml")
        .nth(1)
        .and_then(|s| s.split("```").next())
        .unwrap_or_default();
    let mut section = "";
    let mut lines = toml.lines();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') && !line.contains('=') {
            section = line.trim_matches(|c| c == '[' || c == ']');
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, mut value) = (key.trim(), value.trim().to_owned());
        // Arrays may continue over several lines until the closing bracket.
        if value.starts_with('[') {
            while !value.contains(']') {
                match lines.next() {
                    Some(more) => value.push_str(more.trim()),
                    None => break,
                }
            }
        }
        match (section, key) {
            ("advisory", "id") => a.id = unquote(&value),
            ("advisory", "informational") => a.informational = Some(unquote(&value)),
            ("advisory", "withdrawn") => a.withdrawn = true,
            ("versions", "patched") => {
                a.patched = strings(&value)
                    .iter()
                    .filter_map(|s| Requirement::parse(s))
                    .collect();
            }
            ("versions", "unaffected") => {
                a.unaffected = strings(&value)
                    .iter()
                    .filter_map(|s| Requirement::parse(s))
                    .collect();
            }
            _ => {}
        }
    }
    text.lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or_default()
        .trim()
        .clone_into(&mut a.title);
    a
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').to_owned()
}

/// The quoted strings in a TOML array.
fn strings(array: &str) -> Vec<String> {
    array.split('"').skip(1).step_by(2).map(str::to_owned).collect()
}

/// A semantic version; pre-release and build metadata are kept apart and
/// compared after the numbers, which is enough for advisory ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    pre: String,
}

impl Version {
    fn parse(s: &str) -> Option<Version> {
        let s = s.trim().split('+').next()?;
        let (numbers, pre) = s.split_once('-').map_or((s, ""), |(n, p)| (n, p));
        let mut it = numbers.split('.');
        let major = it.next()?.parse().ok()?;
        let minor = it.next().map_or(Some(0), |x| x.parse().ok())?;
        let patch = it.next().map_or(Some(0), |x| x.parse().ok())?;
        Some(Version {
            major,
            minor,
            patch,
            pre: pre.to_owned(),
        })
    }
}

/// Semantic Versioning 2.0.0, section 11: numbers first; then a pre-release
/// sorts below its release; pre-releases compare identifier by identifier,
/// numeric ones by value and below alphanumeric ones, a shorter list first.
impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => {
                    let (a, b): (Vec<&str>, Vec<&str>) =
                        (self.pre.split('.').collect(), other.pre.split('.').collect());
                    for (x, y) in a.iter().zip(&b) {
                        let order = match (x.parse::<u64>(), y.parse::<u64>()) {
                            (Ok(m), Ok(n)) => m.cmp(&n),
                            (Ok(_), Err(_)) => Ordering::Less,
                            (Err(_), Ok(_)) => Ordering::Greater,
                            (Err(_), Err(_)) => x.cmp(y),
                        };
                        if order != Ordering::Equal {
                            return order;
                        }
                    }
                    a.len().cmp(&b.len())
                }
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre)?;
        }
        Ok(())
    }
}

/// A comma-separated list of comparators that must all hold, such as `>= 1.2, < 2`.
#[derive(Debug, Clone)]
struct Requirement(Vec<(Op, Version, usize)>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Ge,
    Gt,
    Le,
    Lt,
    Eq,
    Caret,
    Tilde,
}

impl Requirement {
    fn parse(s: &str) -> Option<Requirement> {
        let mut parts = Vec::new();
        for c in s.split(',') {
            let c = c.trim();
            let (op, rest) = [
                (">=", Op::Ge),
                ("<=", Op::Le),
                (">", Op::Gt),
                ("<", Op::Lt),
                ("=", Op::Eq),
                ("^", Op::Caret),
                ("~", Op::Tilde),
            ]
            .iter()
            .find_map(|(p, op)| c.strip_prefix(p).map(|r| (*op, r)))
            .unwrap_or((Op::Caret, c));
            let rest = rest.trim();
            let given = rest.split('-').next()?.split('.').count();
            parts.push((op, Version::parse(rest)?, given));
        }
        Some(Requirement(parts))
    }

    fn matches(&self, v: &Version) -> bool {
        self.0.iter().all(|(op, r, given)| match op {
            Op::Ge => v >= r,
            Op::Gt => v > r,
            Op::Le => v <= r,
            Op::Lt => v < r,
            Op::Eq => v == r,
            Op::Caret => v >= r && v < &caret_upper(r),
            Op::Tilde => v >= r && v < &tilde_upper(r, *given),
        })
    }
}

/// `^1.2.3` allows up to `2.0.0`; `^0.2.3` up to `0.3.0`; `^0.0.3` up to `0.0.4`.
fn caret_upper(r: &Version) -> Version {
    let v = |major, minor, patch| Version {
        major,
        minor,
        patch,
        pre: String::new(),
    };
    if r.major > 0 {
        v(r.major + 1, 0, 0)
    } else if r.minor > 0 {
        v(0, r.minor + 1, 0)
    } else {
        v(0, 0, r.patch + 1)
    }
}

/// `~1.2.3` allows up to `1.3.0`; `~1` up to `2.0.0`.
fn tilde_upper(r: &Version, given: usize) -> Version {
    if given <= 1 {
        Version {
            major: r.major + 1,
            minor: 0,
            patch: 0,
            pre: String::new(),
        }
    } else {
        Version {
            major: r.major,
            minor: r.minor + 1,
            patch: 0,
            pre: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn licence_expressions() {
        assert!(licence_allowed("MIT OR Apache-2.0"));
        assert!(licence_allowed("(MIT OR Apache-2.0) AND Unicode-3.0"));
        assert!(licence_allowed("MIT/Apache-2.0"));
        assert!(licence_allowed("Apache-2.0 WITH LLVM-exception OR MIT"));
        assert!(!licence_allowed("GPL-3.0-only"));
        assert!(!licence_allowed("MIT AND GPL-3.0-only"));
        assert!(licence_allowed("GPL-3.0-only OR MIT"));
    }

    #[test]
    fn pre_releases_sort_below_their_release() {
        // The example chain in Semantic Versioning 2.0.0, section 11.
        let chain = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        for pair in chain.windows(2) {
            assert!(v(pair[0]) < v(pair[1]), "{} < {}", pair[0], pair[1]);
        }
        let r = Requirement::parse(">= 1.0.0").unwrap();
        assert!(
            !r.matches(&v("1.0.0-rc.1")),
            "a pre-release is not yet the patched release"
        );
    }

    #[test]
    fn requirements() {
        let r = |s| Requirement::parse(s).unwrap();
        assert!(r(">= 1.2.3").matches(&v("1.2.3")));
        assert!(!r(">= 1.2.3").matches(&v("1.2.2")));
        assert!(r(">= 0.5, < 0.6").matches(&v("0.5.9")));
        assert!(!r(">= 0.5, < 0.6").matches(&v("0.6.0")));
        assert!(r("^0.2.3").matches(&v("0.2.9")));
        assert!(!r("^0.2.3").matches(&v("0.3.0")));
        assert!(r("^1.2").matches(&v("1.9.0")));
        assert!(r("~1.2.3").matches(&v("1.2.9")));
        assert!(!r("~1.2.3").matches(&v("1.3.0")));
        assert!(r("= 1.0.0").matches(&v("1.0.0")));
    }

    #[test]
    fn an_advisory_is_read_from_its_front_matter() {
        let text = "```toml\n[advisory]\nid = \"RUSTSEC-2099-0001\"\npackage = \"demo\"\n\n[versions]\npatched = [\n  \">= 1.4.0\",\n  \">= 1.2.5, < 1.3.0\",\n]\nunaffected = [\"< 1.0.0\"]\n```\n\n# Demo overflow\n";
        let a = parse_advisory(text);
        assert_eq!(a.id, "RUSTSEC-2099-0001");
        assert_eq!(a.title, "Demo overflow");
        assert!(a.affects(&v("1.3.0")));
        assert!(!a.affects(&v("1.2.6")));
        assert!(!a.affects(&v("1.4.1")));
        assert!(!a.affects(&v("0.9.0")));
    }
}

#[cfg(test)]
mod database {
    use super::*;

    /// Reads every advisory in the local copy of the database (fetched by
    /// `cargo xtask deps`) and checks each yields an id, a title and ranges.
    /// Ignored by default because it needs that copy: `cargo test -p xtask -- --ignored`.
    #[test]
    #[ignore = "needs target/advisory-db, fetched by `cargo xtask deps`"]
    fn every_advisory_in_the_database_is_readable() {
        let crates = workspace_root().join("target/advisory-db/crates");
        let mut read = 0;
        let mut without_ranges = Vec::new();
        for dir in std::fs::read_dir(&crates)
            .expect("run `cargo xtask deps` first")
            .filter_map(Result::ok)
        {
            for a in advisories_for(crates.parent().unwrap(), &dir.file_name().to_string_lossy()) {
                assert!(a.id.starts_with("RUSTSEC-"), "{a:?}");
                assert!(!a.title.is_empty(), "{}", a.id);
                if a.patched.is_empty()
                    && a.unaffected.is_empty()
                    && a.informational.is_none()
                    && !a.withdrawn
                {
                    without_ranges.push(a.id);
                }
                read += 1;
            }
        }
        println!(
            "{read} advisories read; {} vulnerabilities with no patched or unaffected range",
            without_ranges.len()
        );
        assert!(read > 500, "only {read} advisories read");
    }
}
