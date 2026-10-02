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
        "signal-pass",
        "signal-refusal",
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
        shown.contains("card-edge      #d4d4d4  {color.card-edge}"),
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
    // The absolute path goes into JSON, so a Windows path's `\` is escaped.
    let outside = std::fs::canonicalize(dir.join("tokens/design.resolver.json")).unwrap();
    let absolute = outside.to_str().unwrap().replace('\\', "\\\\");
    let resolvers = ["../tokens/design.resolver.json".to_owned(), absolute];
    // A symlink needs rights on Windows that a runner may lack; the other
    // two leave the folder there as well.
    #[cfg(unix)]
    let resolvers = {
        std::os::unix::fs::symlink(dir.join("tokens"), inside.join("linked")).unwrap();
        let [up, absolute] = resolvers;
        vec![up, absolute, "linked/design.resolver.json".to_owned()]
    };
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

/// A spec named `<name>.archgram.yaml` (or `.yml`, `.json`) draws
/// `<name>.svg` beside it, and one theme per file as `<name>.light.svg`;
/// any other spec draws its own name with `.svg` (docs/SPEC.md, Formats).
#[test]
fn an_archgram_spec_draws_its_plain_name() {
    let dir = scratch("names");
    let json = read(Path::new(&example("linkshort.json")));
    let yaml =
        "archgram: 1\ntitle: t\ndescription: d\nnodes:\n  - { id: a, kind: service, label: A }\n";
    for (spec, text, drawn) in [
        ("harness.archgram.yaml", yaml, "harness.svg"),
        ("skills.archgram.yml", yaml, "skills.svg"),
        ("shop.archgram.json", json.as_str(), "shop.svg"),
        ("plain.yaml", yaml, "plain.svg"),
    ] {
        std::fs::write(dir.join(spec), text).unwrap();
        let run = archgram(&["build", dir.join(spec).to_str().unwrap()]);
        assert!(
            run.status.success(),
            "{spec}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert!(dir.join(drawn).is_file(), "{spec} should draw {drawn}");
    }
    let run = archgram(&[
        "build",
        dir.join("harness.archgram.yaml").to_str().unwrap(),
        "--split-themes",
    ]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    for drawn in ["harness.light.svg", "harness.dark.svg"] {
        assert!(dir.join(drawn).is_file(), "{drawn}");
    }
}

/// `-o` into a folder that does not exist creates the folder.
#[test]
fn the_output_folder_is_created_when_missing() {
    let dir = scratch("folder");
    let out = dir.join("docs").join("diagrams").join("linkshort.svg");
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
    assert!(read(&out).starts_with("<svg"));
}

/// `archgram spec` prints docs/SPEC.md as this build carries it.
#[test]
fn spec_prints_the_format_it_reads() {
    let run = archgram(&["spec"]);
    assert!(run.status.success());
    let spec = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/SPEC.md"));
    assert_eq!(String::from_utf8_lossy(&run.stdout), spec);
}

/// `archgram` run in `dir`, as CI and the skill run it: from the project's
/// folder, which is where sources may be read (docs/SPEC.md, Sources).
fn archgram_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_archgram"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("the binary runs")
}

const SPEC: &str = "docs/diagrams/clicks.archgram.yaml";

/// A project whose spec, in `docs/diagrams/`, names the code behind its
/// parts and lines (docs/SPEC.md, Sources); `more` adds edges after it.
fn project_with_sources(test: &str, more: &str) -> PathBuf {
    let dir = scratch(test);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("docs/diagrams")).unwrap();
    std::fs::write(
        dir.join("src/worker.rs"),
        "fn count(click: Click) {\n    links.insert(click);\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(SPEC),
        format!(
            "archgram: 1\ntitle: t\ndescription: d\nnodes:\n  - {{ id: worker, kind: service, label: Worker, source: ../../src/worker.rs }}\n  - {{ id: links, kind: database, label: links, source: ../../src }}\nedges:\n  - {{ from: worker, to: links, source: \"../../src/worker.rs#links.insert(\" }}\n{more}"
        ),
    )
    .unwrap();
    dir
}

fn stderr(run: &Output) -> String {
    String::from_utf8_lossy(&run.stderr).into_owned()
}

#[test]
fn check_holds_each_source_to_the_code_from_the_spec_folder() {
    let dir = project_with_sources("sources-found", "");
    let run = archgram_in(&dir, &["check", SPEC]);
    assert!(run.status.success(), "{}", stderr(&run));
    assert!(String::from_utf8_lossy(&run.stdout).contains("(2 nodes, 1 edges, 3 sources found)"));

    let dir = project_with_sources(
        "sources-gone",
        "  - { from: links, to: worker, source: \"../../src/worker.rs#links.delete(\" }\n",
    );
    let run = archgram_in(&dir, &["check", SPEC]);
    assert_eq!(run.status.code(), Some(1));
    assert!(
        stderr(&run).contains(
            "clicks.archgram.yaml:9:40: edge links \u{2192} worker: `../../src/worker.rs` does not hold `links.delete(`"
        ),
        "{}",
        stderr(&run)
    );
}

#[test]
fn build_draws_and_warns_of_each_source_the_code_lacks() {
    let dir = project_with_sources(
        "sources-build",
        "  - { from: links, to: worker, source: ../../src/reader.rs }\n",
    );
    let run = archgram_in(&dir, &["build", SPEC]);
    assert!(run.status.success(), "{}", stderr(&run));
    assert!(
        stderr(&run).contains("archgram: warning: ")
            && stderr(&run).contains("`../../src/reader.rs` does not exist"),
        "{}",
        stderr(&run)
    );
    assert!(dir.join("docs/diagrams/clicks.svg").is_file());
}

/// A source outside the folder archgram runs in is refused by its path
/// alone: a file there and no file there get the same answer, so a spec
/// learns nothing about the disk outside the project.
#[test]
fn a_source_outside_the_project_is_refused_unread() {
    let dir = project_with_sources(
        "sources-outside",
        "  - { from: links, to: worker, source: \"../../../outside.txt#secret\" }\n  - { from: worker, to: worker2, source: \"../../../nothing.txt#secret\" }\n",
    );
    std::fs::write(dir.join("../outside.txt"), "secret").unwrap();
    std::fs::write(
        dir.join(SPEC),
        read(&dir.join(SPEC)).replace(
            "nodes:\n",
            "nodes:\n  - { id: worker2, kind: service, label: W2 }\n",
        ),
    )
    .unwrap();
    let run = archgram_in(&dir, &["check", SPEC]);
    assert_eq!(run.status.code(), Some(1), "{}", stderr(&run));
    let lines: Vec<String> = stderr(&run)
        .lines()
        .filter(|l| l.contains("cannot be read"))
        .map(|l| l.split("cannot be read: ").nth(1).unwrap().to_owned())
        .collect();
    assert_eq!(lines.len(), 2, "{}", stderr(&run));
    assert_eq!(lines[0], lines[1], "both answers alike");
    assert!(lines[0].contains("outside"), "{}", lines[0]);
}

/// A source's file is read only within the size limit, and a name is held
/// to its capitals on disk, so every system answers alike.
#[test]
fn a_large_file_is_not_searched_and_capitals_must_match() {
    let dir = project_with_sources(
        "sources-limits",
        "  - { from: links, to: worker, source: \"../../src/big.bin#needle\" }\n  - { from: worker, to: worker2, source: ../../SRC/worker.rs }\n",
    );
    std::fs::write(
        dir.join(SPEC),
        read(&dir.join(SPEC)).replace(
            "nodes:\n",
            "nodes:\n  - { id: worker2, kind: service, label: W2, source: ../../src/big.bin }\n",
        ),
    )
    .unwrap();
    let big = std::fs::File::create(dir.join("src/big.bin")).unwrap();
    big.set_len(5 * 1024 * 1024).unwrap();
    let run = archgram_in(&dir, &["check", SPEC]);
    let err = stderr(&run);
    assert_eq!(run.status.code(), Some(1), "{err}");
    assert!(
        err.contains("`../../src/big.bin` cannot be searched: it is larger than 4 MiB"),
        "{err}"
    );
    assert!(err.contains("`SRC` is `src` on disk"), "{err}");
    // Cited with no words, the large file is only looked up: one problem
    // for it, not two.
    assert_eq!(err.matches("big.bin").count(), 1, "{err}");
}

/// Symbolic links are not followed, neither to a spec nor to a source, and
/// a FIFO is never opened (it would wait for a writer forever).
#[cfg(unix)]
#[test]
fn links_and_special_files_are_refused() {
    use std::os::unix::fs::symlink;
    let dir = project_with_sources(
        "sources-links",
        "  - { from: links, to: worker, source: \"../../src/passwd#root\" }\n  - { from: worker, to: worker2, source: \"../../src/fifo#abc\" }\n",
    );
    std::fs::write(
        dir.join(SPEC),
        read(&dir.join(SPEC)).replace(
            "nodes:\n",
            "nodes:\n  - { id: worker2, kind: service, label: W2 }\n",
        ),
    )
    .unwrap();
    symlink("/etc/passwd", dir.join("src/passwd")).unwrap();
    let fifo = Command::new("mkfifo")
        .arg(dir.join("src/fifo"))
        .status()
        .unwrap();
    assert!(fifo.success());
    let run = archgram_in(&dir, &["check", SPEC]);
    let err = stderr(&run);
    assert!(err.contains("`passwd` is a symbolic link"), "{err}");
    assert!(
        err.contains("`../../src/fifo` cannot be read: it is neither a file nor a folder"),
        "{err}"
    );

    symlink(
        dir.join(SPEC),
        dir.join("docs/diagrams/linked.archgram.yaml"),
    )
    .unwrap();
    let run = archgram_in(&dir, &["check", "docs/diagrams/linked.archgram.yaml"]);
    assert_eq!(run.status.code(), Some(2));
    assert!(
        stderr(&run).contains("is a symbolic link"),
        "{}",
        stderr(&run)
    );
}

/// A theme in the folder archgram runs in is held to that folder by its
/// path's text: a file outside and no file outside get the same answer.
#[test]
fn a_theme_in_the_current_folder_learns_nothing_outside_it() {
    let dir = scratch("theme-cwd");
    let inside = dir.join("project");
    std::fs::create_dir_all(&inside).unwrap();
    std::fs::write(dir.join("outside.json"), "{}").unwrap();
    let answer = |resolver: &str| {
        std::fs::write(
            inside.join("archgram.theme.json"),
            format!(
                r#"{{"version":1,"resolver":"{resolver}","themes":{{"light":{{"inputs":{{}}}},"dark":{{"inputs":{{}}}}}}}}"#
            ),
        )
        .unwrap();
        let run = archgram_in(&inside, &["theme", "check", "archgram.theme.json"]);
        assert_eq!(run.status.code(), Some(1));
        stderr(&run).replace(resolver, "<resolver>")
    };
    let there = answer("../outside.json");
    assert!(there.contains("the mapping file's folder"), "{there}");
    assert_eq!(there, answer("../nothing.json"));
}

/// The project's folder is the nearest above the spec that holds `.git`, so
/// `check` answers alike from any folder (docs/SPEC.md, Sources).
#[test]
fn sources_are_found_from_any_folder_of_a_project() {
    let dir = project_with_sources("sources-anywhere", "");
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    for (from, spec) in [
        (dir.clone(), SPEC.to_owned()),
        (dir.join("docs"), "diagrams/clicks.archgram.yaml".to_owned()),
        (dir.join("docs/diagrams"), "clicks.archgram.yaml".to_owned()),
    ] {
        let run = archgram_in(&from, &["check", &spec]);
        assert!(run.status.success(), "{}: {}", from.display(), stderr(&run));
    }
    let run = archgram(&["check", dir.join(SPEC).to_str().unwrap()]);
    assert!(run.status.success(), "{}", stderr(&run));
}

/// With no `.git` above the spec, the project is the folder archgram runs
/// in, but never a disk's root, which would make the whole system the
/// project.
#[test]
fn the_whole_disk_is_never_the_project() {
    let dir = project_with_sources("sources-disk", "");
    let root = Path::new("/");
    let run = archgram_in(root, &["check", dir.join(SPEC).to_str().unwrap()]);
    assert_eq!(run.status.code(), Some(1), "{}", stderr(&run));
    assert!(stderr(&run).contains("the whole disk"), "{}", stderr(&run));
}

/// Files that commonly hold secrets, and `.git`, are never read: not as a
/// source, not as a theme's file.
#[test]
fn private_files_are_never_read() {
    let dir = project_with_sources(
        "sources-private",
        "  - { from: links, to: worker, source: \"../../.env#TOKEN=abc\" }\n",
    );
    std::fs::write(dir.join(".env"), "TOKEN=abc").unwrap();
    let run = archgram_in(&dir, &["check", SPEC]);
    assert_eq!(run.status.code(), Some(1));
    assert!(
        stderr(&run).contains("commonly holds secrets"),
        "{}",
        stderr(&run)
    );

    let theme = dir.join("archgram.theme.json");
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    std::fs::write(dir.join(".git/config"), "[core]").unwrap();
    std::fs::write(
        &theme,
        r#"{"version":1,"resolver":".git/config","themes":{"light":{"inputs":{}},"dark":{"inputs":{}}}}"#,
    )
    .unwrap();
    let run = archgram_in(&dir, &["theme", "check", "archgram.theme.json"]);
    assert_eq!(run.status.code(), Some(1));
    assert!(stderr(&run).contains("inside `.git`"), "{}", stderr(&run));
}

/// A link in the middle of a source's path, or above a spec under the
/// folder archgram runs in, is not followed either.
#[cfg(unix)]
#[test]
fn links_on_the_way_are_refused() {
    use std::os::unix::fs::symlink;
    let dir = project_with_sources(
        "sources-midlink",
        "  - { from: links, to: worker, source: \"../../linked/worker.rs#links.insert(\" }\n",
    );
    symlink(dir.join("src"), dir.join("linked")).unwrap();
    let run = archgram_in(&dir, &["check", SPEC]);
    assert!(
        stderr(&run).contains("`linked` is a symbolic link"),
        "{}",
        stderr(&run)
    );

    symlink(dir.join("docs"), dir.join("pages")).unwrap();
    let run = archgram_in(&dir, &["check", "pages/diagrams/clicks.archgram.yaml"]);
    assert_eq!(run.status.code(), Some(2));
    assert!(
        stderr(&run).contains("is a symbolic link"),
        "{}",
        stderr(&run)
    );
}
