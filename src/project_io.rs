use crate::model::Project;
use std::path::{Path, PathBuf};

pub fn save(p: &Project, path: &Path) -> Result<(), String> {
    let _ = (p, path);
    Ok(())
}

pub fn load(path: &Path) -> Result<Project, String> {
    Err(format!("cannot open {}", path.display()))
}

pub fn autosave_path() -> PathBuf {
    std::env::temp_dir().join("cutline-autosave.cutline")
}

pub fn recent() -> Vec<PathBuf> {
    vec![]
}

pub fn push_recent(path: &Path) {
    let _ = path;
}
