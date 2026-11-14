use super::{ExportOptions, JobState};
use crate::model::Project;
use std::path::PathBuf;

pub struct ExportJob {
    state: JobState,
}

impl ExportJob {
    pub fn start(project: &Project, opts: &ExportOptions, out: PathBuf) -> Result<ExportJob, String> {
        let _ = (project, opts, out);
        Err("not implemented".into())
    }

    pub fn poll(&mut self) -> &JobState {
        &self.state
    }

    pub fn cancel(&mut self) {}
}
