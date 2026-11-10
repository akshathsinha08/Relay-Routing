use std::path::{Path, PathBuf};
#[cfg(test)]
use std::path::Component;

use crate::error::AppError;

/// Reduce a user-provided filename to a safe single path component.
pub fn sanitize_filename(name: &str) -> String {
    let normalized = name.replace('\\', "/");
    let base = Path::new(&normalized)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let mut out: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    out = out.trim_matches(['.', ' ']).to_string();
    if out.is_empty() {
        out = "file".to_string();
    }
    if out.len() > 120 {
        let ext = Path::new(&out)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.trim_matches('.')))
            .unwrap_or_default();
        let keep = 120usize.saturating_sub(ext.len());
        out = format!("{}{}", &out[..keep.min(out.len())], ext);
    }
    out
}

/// Verify `candidate` resolves inside `base`. Both are canonicalized, so
/// symlinks cannot escape the root.
pub fn ensure_within(base: &Path, candidate: &Path) -> Result<PathBuf, AppError> {
    let base = base
        .canonicalize()
        .map_err(|_| AppError::invalid("application data directory is unavailable"))?;
    let resolved = candidate
        .canonicalize()
        .map_err(|_| AppError::invalid("file does not exist or is not accessible"))?;
    if !resolved.starts_with(&base) {
        return Err(AppError::new(
            "path_escape",
            "refusing to access a path outside application data",
        ));
    }
    Ok(resolved)
}

/// Build a destination path under `base` from an untrusted display name.
#[cfg(test)]
pub fn contained_destination(base: &Path, filename: &str) -> Result<PathBuf, AppError> {
    let safe = sanitize_filename(filename);
    let dest = base.join(safe);
    if !dest.starts_with(base) {
        return Err(AppError::new("path_escape", "invalid destination path"));
    }
    Ok(dest)
}

#[cfg(test)]
pub fn is_plain_relative(path: &Path) -> bool {
    path.components()
        .all(|c| matches!(c, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_traversal() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("..\\..\\windows"), "windows");
        assert_eq!(sanitize_filename(""), "file");
        assert!(!sanitize_filename(".").contains('/'));
    }

    #[test]
    fn destination_stays_under_base() {
        let base = Path::new("/tmp/claude-multi-test");
        let dest = contained_destination(base, "../../evil.sh").unwrap();
        assert_eq!(dest, base.join("evil.sh"));
    }

    #[test]
    fn relative_only() {
        assert!(is_plain_relative(Path::new("a/b/c.txt")));
        assert!(!is_plain_relative(Path::new("../a")));
    }
}
