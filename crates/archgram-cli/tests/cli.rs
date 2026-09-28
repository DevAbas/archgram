//! The `archgram` command as a user runs it: files in, files out, exit codes.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn archgram(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_archgram"))
        .args(args)
        .output()
        .expect("the binary runs")
}

fn example(name: &str) -> String {
    format!("{}/../../examples/{name}", env!("CARGO_MANIFEST_DIR"))
}

/// A fresh directory for one test's files.
fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("archgram-cli-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn split_themes_writes_one_file_per_theme() {
    let dir = scratch("split");
    let out = dir.join("linkshort.svg");
    let run = archgram(&[
        "build",
        &example("linkshort.json"),
        "-o",
        out.to_str().unwrap(),
        "--split-themes",
    ]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let (light, dark) = (
        read(&dir.join("linkshort.light.svg")),
        read(&dir.join("linkshort.dark.svg")),
    );
    assert!(!dir.join("linkshort.svg").exists());
    for svg in [&light, &dark] {
        assert!(!svg.contains("prefers-color-scheme"), "one theme per file");
    }
    assert_ne!(light, dark);
    // Each is what --theme draws alone.
    let alone = dir.join("alone.svg");
    for (theme, split) in [("light", &light), ("dark", &dark)] {
        let run = archgram(&[
            "build",
            &example("linkshort.json"),
            "-o",
            alone.to_str().unwrap(),
            "--theme",
            theme,
        ]);
        assert!(run.status.success());
        assert_eq!(&read(&alone), split, "{theme}");
    }
}

#[test]
fn split_themes_and_one_theme_do_not_mix() {
    let run = archgram(&[
        "build",
        &example("linkshort.json"),
        "--split-themes",
        "--theme",
        "dark",
    ]);
    assert_eq!(run.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&run.stderr).contains("leave out --theme"));
}

#[test]
fn a_yaml_spec_is_read_and_its_problems_located() {
    let dir = scratch("yaml");
    let spec = dir.join("bad.yaml");
    std::fs::write(
        &spec,
        "archgram: 1\ntitle: t\ndescription: d\nnodes:\n  - { id: a, kind: service, label: A, lable: B }\n",
    )
    .unwrap();
    let run = archgram(&["check", spec.to_str().unwrap()]);
    assert_eq!(run.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("bad.yaml:5:"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let run = archgram(&["check", &example("linkshort.json").replace(".json", ".txt")]);
    assert_eq!(run.status.code(), Some(2));
}

/// A copy of the folder `from` at `to`, files and folders.
fn copy_folder(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_folder(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

/// A mapping of archgram's own tokens, every role to its own token, in
/// `dir`, with a copy of the tokens beside it.
fn own_theme(dir: &Path, extra_role: &str) -> PathBuf {
    let tokens = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../design-system/tokens");
    copy_folder(&tokens, &dir.join("tokens"));
    own_mapping(dir, "tokens/design.resolver.json", extra_role)
}

/// The mapping of archgram's own tokens in `dir`, naming `resolver`.
fn own_mapping(dir: &Path, resolver: &str, extra_role: &str) -> PathBuf {
    let roles = [
        "badge",
        "canvas",
        "card",
        "card-edge",
        "connector",
        "frame",
        "icon-ai",
        "icon-build",
        "icon-client",
        "icon-core",
        "signal-core",
        "text",
        "text-muted",
    ]
    .iter()
    .map(|r| format!(r#""{r}": "color.{r}""#))
    .collect::<Vec<_>>()
    .join(", ");
    let mapping = format!(
        r#"{{ "version": 1, "resolver": "{resolver}", "roles": {{ {roles}{extra_role} }},
        "themes": {{ "light": {{ "inputs": {{ "theme": "light" }} }}, "dark": {{ "inputs": {{ "theme": "dark" }} }} }} }}"#
    );
    let path = dir.join("archgram.theme.json");
    std::fs::write(&path, mapping).unwrap();
    path
}

#[test]
fn a_theme_file_is_checked_and_drawn_with() {
    let dir = scratch("theme");
    let theme = own_theme(&dir, "");
    let run = archgram(&["theme", "check", theme.to_str().unwrap()]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let shown = String::from_utf8_lossy(&run.stdout);
    assert!(
        shown.contains("card-edge    #d4d4d4  {color.card-edge}"),
        "{shown}"
    );
    // archgram's own tokens draw what the built-in palette draws.
    let (with, without) = (dir.join("with.svg"), dir.join("without.svg"));
    let spec = example("kinds.json");
    let a = archgram(&[
        "build",
        &spec,
        "-o",
        with.to_str().unwrap(),
        "--theme-file",
        theme.to_str().unwrap(),
    ]);
    let b = archgram(&["build", &spec, "-o", without.to_str().unwrap()]);
    assert!(a.status.success() && b.status.success());
    assert!(
        read(&with) == read(&without),
        "the own tokens draw differently"
    );
}

#[test]
fn a_broken_theme_file_is_a_problem_not_a_drawing() {
    let dir = scratch("theme-broken");
    let theme = own_theme(&dir, r#", "paper": "color.card""#);
    let run = archgram(&["theme", "check", theme.to_str().unwrap()]);
    assert_eq!(run.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&run.stderr)
            .contains("/roles/paper: `paper` is not an archgram role")
    );
    let out = dir.join("never.svg");
    let run = archgram(&[
        "build",
        &example("kinds.json"),
        "-o",
        out.to_str().unwrap(),
        "--theme-file",
        theme.to_str().unwrap(),
    ]);
    assert_eq!(run.status.code(), Some(1));
    assert!(!out.exists());
}

/// A spec's own text printed back in a problem cannot drive the terminal:
/// its control characters are written as escapes.
#[test]
fn problems_print_control_characters_as_escapes() {
    let dir = scratch("escapes");
    let spec = dir.join("spec.json");
    std::fs::write(
        &spec,
        r#"{ "archgram": 1, "title": "t", "description": "d", "palette": "x\u001b[2J",
            "nodes": [{ "id": "a", "kind": "service", "label": "A" }] }"#,
    )
    .unwrap();
    let run = archgram(&["check", spec.to_str().unwrap()]);
    assert_eq!(run.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(!stderr.contains('\u{1b}'), "{stderr}");
    assert!(stderr.contains(r"unknown palette `x\u{1b}[2J`"), "{stderr}");
}

/// The drawing replaces a symlink at its path and never writes through it,
/// so a spec's repository cannot aim it at another file.
#[cfg(unix)]
#[test]
fn a_symlink_at_the_output_is_replaced_not_followed() {
    let dir = scratch("symlink");
    let elsewhere = dir.join("elsewhere.txt");
    std::fs::write(&elsewhere, "keep me").unwrap();
    let out = dir.join("diagram.svg");
    std::os::unix::fs::symlink(&elsewhere, &out).unwrap();
    let run = archgram(&[
        "build",
        &example("linkshort.json"),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(read(&elsewhere), "keep me");
    assert!(!std::fs::symlink_metadata(&out).unwrap().is_symlink());
    assert!(read(&out).starts_with("<svg"));
    let left: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left.len(), 2, "no temporary file left: {left:?}");
}

/// A theme reads only the files under its mapping's folder: `..`, an
/// absolute path and a symlink cannot lead it out.
#[test]
fn a_theme_reads_only_under_its_folder() {
    let dir = scratch("theme-folder");
    own_theme(&dir, "");
    let inside = dir.join("project");
    std::fs::create_dir_all(&inside).unwrap();
    let outside = std::fs::canonicalize(dir.join("tokens/design.resolver.json")).unwrap();
    let mut resolvers = vec![
        "../tokens/design.resolver.json".to_owned(),
        outside.to_str().unwrap().to_owned(),
    ];
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.join("tokens"), inside.join("linked")).unwrap();
        resolvers.push("linked/design.resolver.json".to_owned());
    }
    for resolver in resolvers {
        let theme = own_mapping(&inside, &resolver, "");
        let run = archgram(&["theme", "check", theme.to_str().unwrap()]);
        assert_eq!(run.status.code(), Some(1), "{resolver}");
        let stderr = String::from_utf8_lossy(&run.stderr);
        assert!(
            stderr.contains("the mapping file's folder"),
            "{resolver}: {stderr}"
        );
    }
}
