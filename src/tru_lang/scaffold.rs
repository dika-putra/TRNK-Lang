//! `tru_id new <name>` — project boilerplate generator.
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

const MANIFEST: &str = include_str!("templates/trn.toml");
const MAIN: &str = include_str!("templates/main.tru");
const MODULES: [(&str, &str); 4] = [
    ("fetch", include_str!("templates/fetch.tru")),
    ("train", include_str!("templates/train.tru")),
    ("inference", include_str!("templates/inference.tru")),
    ("pipe", include_str!("templates/pipe.tru")),
];

/// Creates `<parent>/<name>/` with a manifest, `src/main.tru` and the
/// fetch / train / inference / pipe modules. Refuses to touch an existing path.
pub fn new_project(name: &str, parent: &Path) -> std::io::Result<PathBuf> {
    let valid = !name.is_empty()
        && !name.starts_with('-')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !valid || is_reserved_windows_name(name) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "project name may only contain letters, numbers, '_' and '-', and cannot be a reserved name like 'con' or 'nul'",
        ));
    }
    let root = parent.join(name);
    // create_dir (not create_dir_all) so an existing path is reported by the OS, without a check-then-create race.
    std::fs::create_dir(&root).map_err(|e| {
        if e.kind() == ErrorKind::AlreadyExists {
            Error::new(ErrorKind::AlreadyExists, format!("{} already exists", root.display()))
        } else {
            e
        }
    })?;
    let modules = root.join("src").join("modules");
    std::fs::create_dir_all(&modules)?;
    std::fs::write(root.join("trn.toml"), MANIFEST.replace("{{name}}", name))?;
    std::fs::write(root.join("src").join("main.tru"), MAIN.replace("{{name}}", name))?;
    for (module, body) in MODULES {
        std::fs::write(modules.join(format!("{module}.tru")), body)?;
    }
    Ok(root)
}

/// Windows rejects these as file names, so a project called `con` could not be created there.
fn is_reserved_windows_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    matches!(n.as_str(), "con" | "prn" | "aux" | "nul")
        || ((n.starts_with("com") || n.starts_with("lpt"))
            && n.len() == 4
            && matches!(n.as_bytes()[3], b'1'..=b'9'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tru_lang::{run_file, Value};

    /// Temp directory that is removed on drop, also when an assert fails.
    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    fn temp_dir(tag: &str) -> TempDir {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tru_new_{tag}_{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    #[test]
    fn generated_project_runs_the_pipeline() {
        let parent = temp_dir("run");
        let root = new_project("demo", &parent.0).unwrap();
        for f in ["trn.toml", "src/main.tru", "src/modules/fetch.tru", "src/modules/train.tru",
                  "src/modules/inference.tru", "src/modules/pipe.tru"] {
            assert!(root.join(f).is_file(), "missing {f}");
        }
        // main.tru ends with a call to main(), which returns unit; probe the pipeline directly.
        std::fs::write(
            root.join("src/probe.tru"),
            "use modules::fetch;\nuse modules::train;\nuse modules::inference;\nuse modules::pipe;\npipe_run(\"data\", \"hi\")\n",
        ).unwrap();
        match run_file(root.join("src/probe.tru")).unwrap() {
            Value::Str(s) => assert_eq!(s, "inference(model(raw data from data), hi)"),
            other => panic!("unexpected value: {other}"),
        }
        run_file(root.join("src/main.tru")).unwrap();
    }

    #[test]
    fn rejects_bad_names_and_existing_dirs() {
        let parent = temp_dir("bad");
        for name in ["", "-a", "a b", "../x", "a/b", "con", "NUL", "com1", "lpt9"] {
            assert!(new_project(name, &parent.0).is_err(), "accepted {name:?}");
        }
        new_project("once", &parent.0).unwrap();
        let err = new_project("once", &parent.0).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::AlreadyExists);
    }

    #[test]
    fn missing_module_file_is_an_error() {
        let parent = temp_dir("missing");
        let root = new_project("demo", &parent.0).unwrap();
        std::fs::remove_file(root.join("src/modules/train.tru")).unwrap();
        let err = run_file(root.join("src/main.tru")).unwrap_err().to_string();
        assert!(err.contains("train.tru"), "error should name the file: {err}");
    }

    #[test]
    fn malformed_module_import_is_an_error() {
        let parent = temp_dir("badimport");
        let root = new_project("demo", &parent.0).unwrap();
        std::fs::write(root.join("src/bad.tru"), "use modules::fetch; // note\n").unwrap();
        let err = run_file(root.join("src/bad.tru")).unwrap_err().to_string();
        assert!(err.contains("invalid module import"), "{err}");
    }
}
