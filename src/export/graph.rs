use super::ExportOptions;
use crate::model::*;
use std::path::Path;

pub fn export_args(project: &Project, opts: &ExportOptions, out: &Path) -> Result<Vec<String>, String> {
    let _ = (project, opts, out);
    Err("not implemented".into())
}

pub fn audio_preview_args(project: &Project, from: f64, sample_rate: u32, channels: u16) -> Option<Vec<String>> {
    let _ = (project, from, sample_rate, channels);
    None
}
