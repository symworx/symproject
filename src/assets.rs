// Copyright (c) 2026, Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

use std::{
    env,
    fs,
    path::{
        Path,
        PathBuf,
    },
};

use include_dir::{
    Dir,
    include_dir,
};

use crate::error::{
    Error,
    Result,
};

static EMBEDDED_CATALOG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/catalog.yaml"));
static EMBEDDED_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// A project kit root has `catalog.yaml` and `templates/`.
pub fn is_root(dir: &Path) -> bool {
    dir.join("catalog.yaml").is_file() && dir.join("templates").is_dir()
}

/// Prefer `SYMPROJECT_ROOT`, then a checkout next to the executable or the
/// working directory, then the versioned cache filled from the binary.
pub fn find_root() -> Result<PathBuf> {
    if let Ok(raw) = env::var("SYMPROJECT_ROOT") {
        let p = PathBuf::from(raw);
        if is_root(&p) {
            return Ok(fs::canonicalize(&p).unwrap_or(p));
        }
        return Err(Error::CatalogMissing(p.join("catalog.yaml")));
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.to_path_buf());
        }
    }
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd);
    }

    for start in candidates {
        let mut cur = Some(start.as_path());
        while let Some(dir) = cur {
            if is_root(dir) {
                return Ok(fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf()));
            }
            cur = dir.parent();
        }
    }

    ensure_embedded()
}

/// `$XDG_DATA_HOME/symproject/<crate version>/` (or `~/.local/share/symproject/<version>/`).
pub fn embedded_dir() -> PathBuf {
    data_dir().join(env!("CARGO_PKG_VERSION"))
}

fn data_dir() -> PathBuf {
    if let Some(p) = env::var_os("SYMPROJECT_DATA") {
        return PathBuf::from(p);
    }
    if let Some(p) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(p).join("symproject");
    }
    #[cfg(windows)]
    {
        if let Some(p) = env::var_os("LOCALAPPDATA") {
            return PathBuf::from(p).join("symproject");
        }
    }
    if let Some(p) = env::var_os("HOME") {
        return PathBuf::from(p).join(".local/share/symproject");
    }
    env::temp_dir().join("symproject")
}

fn ensure_embedded() -> Result<PathBuf> {
    let dest = embedded_dir();
    if is_root(&dest) {
        return Ok(dest);
    }
    extract_embedded(&dest)?;
    if is_root(&dest) {
        Ok(dest)
    } else {
        Err(Error::RootNotFound)
    }
}

fn extract_embedded(dest: &Path) -> Result<()> {
    let templates = dest.join("templates");
    // include_dir writes root files without creating `templates/` first.
    fs::create_dir_all(&templates)?;
    EMBEDDED_TEMPLATES.extract(&templates)?;
    fs::write(dest.join("catalog.yaml"), EMBEDDED_CATALOG)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_embedded_has_no_lab_brand(dir: &Path) {
        let mut pending = vec![dir.to_path_buf()];
        while let Some(current) = pending.pop() {
            for entry in fs::read_dir(&current).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                let bytes = fs::read(&path).unwrap();
                let Ok(text) = String::from_utf8(bytes) else {
                    continue;
                };
                assert!(!text.to_lowercase().contains("csymd"), "{}", path.display());
            }
        }
    }

    #[test]
    fn embedded_extract_is_root_and_keeps_paper_templates() {
        let dir = tempfile::tempdir().unwrap();
        extract_embedded(dir.path()).unwrap();
        assert!(is_root(dir.path()));
        let readme = dir.path().join("templates/shapes/paper-line/README.md.tpl");
        let claims = dir.path().join("templates/paper/notes/claims.md.tpl");
        assert!(readme.is_file(), "missing {}", readme.display());
        assert!(claims.is_file(), "missing {}", claims.display());
        assert!(dir.path().join("templates/lab.yml").is_file());
        assert_embedded_has_no_lab_brand(dir.path());
    }
}
