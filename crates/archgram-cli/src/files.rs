//! How the `archgram` command reads a file (SECURITY.md, What archgram
//! reads). A spec, a theme and the sources a spec names may all come from
//! someone else, such as a pull request that CI checks, so each file is a
//! regular file, opened only once its type is known, read up to [`LIMIT`]
//! bytes, and never reached through a symbolic link that file's author
//! could have placed. A source is read only from under the folder archgram
//! runs in, judged by its path's text before the disk is touched, so a
//! spec cannot learn even whether a file outside it exists.

use std::io::Read;
use std::path::{Component, Path, PathBuf};

use archgram_core::sources::Found;

/// The most bytes archgram reads from one file: far more than any spec,
/// token file or source file holds, and far less than would slow a run.
pub const LIMIT: u64 = 4 * 1024 * 1024;

fn too_large() -> String {
    format!("it is larger than {} MiB", LIMIT / (1024 * 1024))
}

/// The bytes of the regular file at `path`, at most [`LIMIT`] of them. Its
/// type is known before it is opened, so a FIFO or a device is never
/// opened, and is checked again on the open file.
pub fn read_regular(path: &Path) -> Result<Vec<u8>, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("it is not a file".into());
    }
    if meta.len() > LIMIT {
        return Err(too_large());
    }
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("it is not a file".into());
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err(too_large());
    }
    Ok(bytes)
}

/// `path` with `.` and `..` resolved by its text alone, as the system
/// resolves them where no symbolic link is on the way. A `..` above the
/// start of a relative path is kept; above the root, it stays at the root.
pub fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => match out.components().next_back() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => out.push(".."),
            },
            other => out.push(other),
        }
    }
    out
}

/// The symbolic link a spec or theme file is reached through, if any: the
/// file itself, or, when it is under `root`, any folder between `root` and
/// it, since those may come with the file (a pull request can add a link).
pub fn link_to(root: &Path, path: &Path) -> Option<PathBuf> {
    let at = lexical(&root.join(path));
    let is_link = |p: &Path| std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink());
    if let Ok(below) = at.strip_prefix(root) {
        let mut here = root.to_path_buf();
        for part in below.components() {
            here.push(part);
            if is_link(&here) {
                return Some(here);
            }
        }
        None
    } else {
        is_link(&at).then_some(at)
    }
}

/// What is at a source's `path`, written from `folder` (the spec's), when
/// archgram runs in `root`: only under `root`, through no symbolic link,
/// each name with the capitals it has on disk; and the file's text only
/// when `words` asks for it (docs/SPEC.md, Sources).
pub fn find_source(root: &Path, folder: &Path, path: &str, words: bool) -> Found {
    let at = lexical(&folder.join(path));
    let Ok(below) = at.strip_prefix(root) else {
        return Found::Unreadable(format!(
            "it is outside {}, the folder archgram runs in, and archgram reads sources only under it; run archgram from the project's folder",
            root.display()
        ));
    };
    let mut here = root.to_path_buf();
    for part in below.components() {
        let name = part.as_os_str();
        // The folder's own names, so a system that ignores capitals (macOS,
        // Windows) answers as one that does not.
        let names: Vec<_> = match std::fs::read_dir(&here) {
            Ok(entries) => entries.flatten().map(|e| e.file_name()).collect(),
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                return Found::Nothing;
            }
            Err(e) => return Found::Unreadable(e.to_string()),
        };
        if !names.iter().any(|n| n == name) {
            let wanted = name.to_string_lossy();
            return match names
                .iter()
                .find(|n| n.to_string_lossy().eq_ignore_ascii_case(&wanted))
            {
                Some(on_disk) => Found::Unreadable(format!(
                    "`{wanted}` is `{}` on disk; write it with the same capitals, so every system finds it alike",
                    on_disk.to_string_lossy()
                )),
                None => Found::Nothing,
            };
        }
        here.push(part);
        match std::fs::symlink_metadata(&here) {
            Ok(m) if m.file_type().is_symlink() => {
                return Found::Unreadable(format!(
                    "`{}` is a symbolic link, which archgram does not follow; name the file it leads to",
                    name.to_string_lossy()
                ));
            }
            Ok(_) => {}
            Err(e) => return Found::Unreadable(e.to_string()),
        }
    }
    match std::fs::symlink_metadata(&here) {
        Ok(m) if m.is_dir() => Found::Folder,
        Ok(m) if m.is_file() && !words => Found::File(None),
        Ok(m) if m.is_file() => Found::File(Some(
            read_regular(&here).map(|bytes| String::from_utf8_lossy(&bytes).into_owned()),
        )),
        Ok(_) => Found::Unreadable("it is neither a file nor a folder".into()),
        Err(e) => Found::Unreadable(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dots_resolve_by_text() {
        assert_eq!(
            lexical(Path::new("/p/docs/diagrams/../../src/./a.rs")),
            Path::new("/p/src/a.rs")
        );
        assert_eq!(lexical(Path::new("/p/../../../etc")), Path::new("/etc"));
        assert_eq!(lexical(Path::new("a/../../b")), Path::new("../b"));
        assert_eq!(lexical(Path::new("../../b/./c/..")), Path::new("../../b"));
    }
}
