//! Project context — file tree awareness for code indexing.
//!
//! Scans a project directory for source files and provides
//! context about the project structure.

use std::path::{Path, PathBuf};

/// Information about a source file in the project.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub relative_path: String,
    pub extension: String,
    pub size_bytes: u64,
}

/// Scan a directory for Rust source files.
pub fn scan_rust_files(root: &Path) -> Vec<SourceFile> {
    let mut files = Vec::new();
    scan_recursive(root, root, &mut files);
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    files
}

fn scan_recursive(root: &Path, dir: &Path, files: &mut Vec<SourceFile>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();

        // Skip hidden directories and target/
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
        }

        if path.is_dir() {
            scan_recursive(root, &path, files);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            files.push(SourceFile {
                path: path.clone(),
                relative_path: relative,
                extension: "rs".to_string(),
                size_bytes: size,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_finds_rust_files() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let files = scan_rust_files(root);
        assert!(!files.is_empty(), "Should find .rs files in the project");
        assert!(files.iter().any(|f| f.relative_path.contains("colony.rs")));
    }
}
