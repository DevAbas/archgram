//! `cargo xtask npm`: the npm packages from the built binaries
//! (ARCHITECTURE.md, Distribution), in `target/npm/`.
//!
//! - `archgram`: the launcher from `packages/archgram/`, its version the
//!   workspace's, every platform package an optional dependency of that
//!   same version;
//! - `@archgram/cli-<os>-<cpu>`: one per platform, holding only its binary,
//!   declaring its `os` and `cpu` so npm installs just the one that fits.
//!
//! A platform's binary is read from `target/<rust target>/release/`, or
//! `target/release/` for this machine's own. A missing binary leaves its
//! package out, unless `--all` asks for every one (a release). `--smoke`
//! then runs the launcher's tests and installs the packages for this
//! machine into a scratch project, where `archgram build` must write the
//! same bytes as the binary run directly.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::{Value, json};

/// Each platform: its npm key (`process.platform-process.arch`), its `os`
/// and `cpu`, and the Rust target its binary is built for. Linux links
/// musl statically, so one binary runs on every distribution.
pub const PLATFORMS: [(&str, &str, &str, &str); 6] = [
    ("darwin-arm64", "darwin", "arm64", "aarch64-apple-darwin"),
    ("darwin-x64", "darwin", "x64", "x86_64-apple-darwin"),
    ("linux-arm64", "linux", "arm64", "aarch64-unknown-linux-musl"),
    ("linux-x64", "linux", "x64", "x86_64-unknown-linux-musl"),
    ("win32-arm64", "win32", "arm64", "aarch64-pc-windows-msvc"),
    ("win32-x64", "win32", "x64", "x86_64-pc-windows-msvc"),
];

/// The packages' author, as npm writes a person: name and site.
const AUTHOR: &str = "Abas Turabli (https://abasturabli.com)";

/// Where the packages are built from; trusted publishing and provenance
/// check it against the workflow's repository, character for character.
const REPOSITORY: &str = "git+https://github.com/DevAbas/archgram.git";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The workspace's version, from `[workspace.package]`.
fn version() -> Result<String, String> {
    let manifest = std::fs::read_to_string(root().join("Cargo.toml")).map_err(|e| e.to_string())?;
    let section = manifest
        .split("[workspace.package]")
        .nth(1)
        .ok_or("Cargo.toml has no [workspace.package]")?;
    section
        .lines()
        .find_map(|l| l.trim().strip_prefix("version = \""))
        .and_then(|v| v.strip_suffix('"'))
        .map(str::to_owned)
        .ok_or_else(|| "[workspace.package] has no version".into())
}

/// This machine's npm key, when archgram is built for it.
fn host() -> Option<&'static str> {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "win32",
        _ => return None,
    };
    let cpu = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        _ => return None,
    };
    PLATFORMS.iter().find(|p| p.1 == os && p.2 == cpu).map(|p| p.0)
}

fn exe(os: &str) -> &'static str {
    if os == "win32" { "archgram.exe" } else { "archgram" }
}

/// Where a platform's built binary is, if it has been built.
fn built(key: &str, os: &str, target: &str) -> Option<PathBuf> {
    let own = root().join("target").join(target).join("release").join(exe(os));
    if own.exists() {
        return Some(own);
    }
    let host_build = root().join("target/release").join(exe(os));
    (host() == Some(key) && host_build.exists()).then_some(host_build)
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())? + "\n";
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn copy(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| format!("{} to {}: {e}", from.display(), to.display()))
}

/// Writes the packages; returns the platform keys that have one.
fn assemble(all: bool) -> Result<Vec<&'static str>, String> {
    let version = version()?;
    let out = root().join("target/npm");
    if out.exists() {
        std::fs::remove_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    let mut written = Vec::new();
    for &(key, os, cpu, target) in &PLATFORMS {
        let Some(binary) = built(key, os, target) else {
            if all {
                return Err(format!("no binary for {key}: build `--target {target}` first"));
            }
            println!("{key}: no binary in target/{target}/release, left out");
            continue;
        };
        let dir = out.join(format!("cli-{key}"));
        copy(&binary, &dir.join("bin").join(exe(os)))?;
        write_json(
            &dir.join("package.json"),
            &json!({
                "name": format!("@archgram/cli-{key}"),
                "version": version,
                "description": format!("The archgram binary for {os} {cpu}. Install `archgram`, which picks this one."),
                "author": AUTHOR,
                // The binary carries Geist (OFL) and Simple Icons' data (CC0).
                "license": "MIT AND OFL-1.1 AND CC0-1.0",
                "repository": { "type": "git", "url": REPOSITORY },
                "homepage": "https://github.com/DevAbas/archgram#readme",
                "bugs": { "url": "https://github.com/DevAbas/archgram/issues" },
                "os": [os],
                "cpu": [cpu],
                "files": ["bin", "THIRD-PARTY-LICENSES"],
                "preferUnplugged": true,
                // Scoped packages publish as restricted unless told otherwise.
                "publishConfig": { "access": "public" },
            }),
        )?;
        copy(&root().join("LICENSE"), &dir.join("LICENSE"))?;
        copy(
            &root().join("THIRD-PARTY-LICENSES"),
            &dir.join("THIRD-PARTY-LICENSES"),
        )?;
        std::fs::write(
            dir.join("README.md"),
            format!("# @archgram/cli-{key}\n\nThe `archgram` binary for {os} {cpu}. Install `archgram` instead; it depends on this package on this platform only.\n"),
        )
        .map_err(|e| e.to_string())?;
        println!("{key}: {}", binary.display());
        written.push(key);
    }
    // The launcher: its manifest with the release's version, and every
    // platform package, of that version, as an optional dependency.
    let source = root().join("packages/archgram");
    let dir = out.join("archgram");
    copy(&source.join("bin/archgram.js"), &dir.join("bin/archgram.js"))?;
    copy(&source.join("lib/platform.js"), &dir.join("lib/platform.js"))?;
    copy(&source.join("README.md"), &dir.join("README.md"))?;
    copy(&root().join("LICENSE"), &dir.join("LICENSE"))?;
    let mut manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(source.join("package.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    manifest["version"] = json!(version);
    manifest["optionalDependencies"] = PLATFORMS
        .iter()
        .map(|p| (format!("@archgram/cli-{}", p.0), json!(version)))
        .collect::<serde_json::Map<_, _>>()
        .into();
    write_json(&dir.join("package.json"), &manifest)?;
    println!("archgram {version}: target/npm");
    Ok(written)
}

fn run(dir: &Path, program: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} {} failed:\n{}{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}

/// The launcher's tests, then an install of this machine's packages into a
/// scratch project, where the command must draw what the binary draws.
fn smoke(written: &[&str]) -> Result<(), String> {
    let root = root();
    run(
        &root.join("packages/archgram"),
        "node",
        &["--test", "lib/platform.test.js"],
    )?;
    println!("launcher tests: passed");
    let key = host().ok_or("this machine has no platform package")?;
    if !written.contains(&key) {
        return Err(format!("no package for this machine ({key}); build it first"));
    }
    let scratch = root.join("target/npm-smoke");
    if scratch.exists() {
        std::fs::remove_dir_all(&scratch).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
    let pack = |dir: &str| -> Result<PathBuf, String> {
        let name = run(
            &root.join("target/npm").join(dir),
            "npm",
            &[
                "pack",
                "--silent",
                "--pack-destination",
                scratch.to_str().unwrap_or("."),
            ],
        )?;
        Ok(scratch.join(String::from_utf8_lossy(&name).trim()))
    };
    let (launcher, platform) = (pack("archgram")?, pack(&format!("cli-{key}"))?);
    write_json(
        &scratch.join("package.json"),
        &json!({ "name": "smoke", "private": true }),
    )?;
    // The platform package from its tarball, the launcher without fetching
    // the other platforms' packages from the registry.
    run(
        &scratch,
        "npm",
        &[
            "install",
            "--no-audit",
            "--no-fund",
            "--omit=optional",
            platform.to_str().unwrap_or_default(),
            launcher.to_str().unwrap_or_default(),
        ],
    )?;
    let spec = root.join("examples/cv-screener-architecture.json");
    let (via_npm, direct) = (scratch.join("npm.svg"), scratch.join("direct.svg"));
    run(
        &scratch,
        "npx",
        &[
            "archgram",
            "build",
            spec.to_str().unwrap_or_default(),
            "-o",
            via_npm.to_str().unwrap_or_default(),
        ],
    )?;
    let binary = scratch
        .join("node_modules/@archgram")
        .join(format!("cli-{key}"))
        .join("bin")
        .join(exe(if key.starts_with("win32") { "win32" } else { "" }));
    run(
        &scratch,
        binary.to_str().unwrap_or_default(),
        &[
            "build",
            spec.to_str().unwrap_or_default(),
            "-o",
            direct.to_str().unwrap_or_default(),
        ],
    )?;
    let (a, b) = (
        std::fs::read(&via_npm).map_err(|e| e.to_string())?,
        std::fs::read(&direct).map_err(|e| e.to_string())?,
    );
    if a != b {
        return Err("`npx archgram` drew different bytes from the binary".into());
    }
    println!("npx archgram build: the same {} bytes as the binary", a.len());
    Ok(())
}

/// `cargo xtask npm [--all] [--smoke]`.
pub fn npm(flags: &[&str]) -> ExitCode {
    let all = flags.contains(&"--all");
    let result = assemble(all).and_then(|written| {
        if flags.contains(&"--smoke") {
            smoke(&written)
        } else {
            Ok(())
        }
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask npm: {e}");
            ExitCode::FAILURE
        }
    }
}
