use super::graph::export_args;
use super::{ExportOptions, Format, JobState};
use crate::ffmpeg;
use crate::model::Project;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

#[derive(Default, Debug, PartialEq)]
struct Progress {
    out_secs: f64,
    speed: Option<f32>,
    ended: bool,
}

impl Progress {
    fn feed(&mut self, line: &str) {
        let Some((key, value)) = line.trim().split_once('=') else { return };
        let value = value.trim();
        match key {
            "out_time_us" | "out_time_ms" => {
                if let Ok(us) = value.parse::<i64>() {
                    self.out_secs = us.max(0) as f64 / 1e6;
                }
            }
            "speed" => self.speed = value.trim_end_matches('x').trim().parse().ok(),
            "progress" => self.ended = value == "end",
            _ => {}
        }
    }
}

pub struct ExportJob {
    state: JobState,
    child: Child,
    out: PathBuf,
    duration: f64,
    started: Instant,
    progress: Arc<Mutex<Progress>>,
    stderr: Arc<Mutex<VecDeque<String>>>,
    threads: Vec<JoinHandle<()>>,
}

impl ExportJob {
    pub fn start(project: &Project, opts: &ExportOptions, out: PathBuf) -> Result<ExportJob, String> {
        let duration = opts.range.map(|(a, b)| b - a).unwrap_or_else(|| project.duration());
        if duration <= 0.0 {
            return Err("The project is empty".into());
        }
        if !ffmpeg::available() {
            return Err("ffmpeg was not found".into());
        }
        if !out.parent().map(|p| p.as_os_str().is_empty() || p.is_dir()).unwrap_or(false) {
            return Err("The output folder does not exist".into());
        }
        if project.media.iter().any(|m| m.path == out) {
            return Err("The output file would overwrite a source media file".into());
        }
        let args = export_args(project, opts, &out)?;
        let mut child = ffmpeg::command("ffmpeg")
            .args(["-y", "-nostats", "-progress", "pipe:1"])
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Could not start ffmpeg: {e}"))?;
        let progress = Arc::new(Mutex::new(Progress::default()));
        let stderr = Arc::new(Mutex::new(VecDeque::new()));
        let mut threads = vec![];
        if let Some(pipe) = child.stdout.take() {
            let progress = progress.clone();
            threads.push(std::thread::spawn(move || {
                for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                    progress.lock().unwrap().feed(&line);
                }
            }));
        }
        if let Some(pipe) = child.stderr.take() {
            let tail = stderr.clone();
            threads.push(std::thread::spawn(move || {
                for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                    let mut tail = tail.lock().unwrap();
                    if tail.len() == 40 {
                        tail.pop_front();
                    }
                    tail.push_back(line);
                }
            }));
        }
        let state = JobState::Running { progress: 0.0, eta: None, speed: None };
        Ok(ExportJob { state, child, out, duration, started: Instant::now(), progress, stderr, threads })
    }

    pub fn poll(&mut self) -> &JobState {
        if !matches!(self.state, JobState::Running { .. }) {
            return &self.state;
        }
        match self.child.try_wait() {
            Ok(None) => {
                let p = self.progress.lock().unwrap();
                let fraction = (p.out_secs / self.duration).clamp(0.0, 1.0);
                let elapsed = self.started.elapsed().as_secs_f64();
                let eta = (fraction > 0.01).then(|| elapsed / fraction * (1.0 - fraction));
                self.state = JobState::Running { progress: fraction as f32, eta, speed: p.speed };
            }
            Ok(Some(status)) if status.success() => {
                self.state = JobState::Done { path: self.out.clone(), secs: self.started.elapsed().as_secs_f64() };
            }
            Ok(Some(_)) | Err(_) => {
                for t in self.threads.drain(..) {
                    let _ = t.join();
                }
                let tail = self.stderr.lock().unwrap();
                let lines: Vec<&str> = tail.iter().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
                let from = lines.len().saturating_sub(6);
                let msg = lines[from..].join("\n");
                self.state = JobState::Failed(if msg.is_empty() { "ffmpeg failed".into() } else { msg });
                let _ = std::fs::remove_file(&self.out);
            }
        }
        &self.state
    }

    pub fn cancel(&mut self) {
        if !matches!(self.state, JobState::Running { .. }) {
            return;
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.out);
        self.state = JobState::Cancelled;
    }
}

impl Drop for ExportJob {
    fn drop(&mut self) {
        if matches!(self.state, JobState::Running { .. }) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub fn export_frame(project: &Project, t: f64, out: &Path) -> Result<(), String> {
    let s = &project.settings;
    let opts = ExportOptions {
        format: Format::Mp4H264,
        width: s.width & !1,
        height: s.height & !1,
        fps: s.fps,
        quality: 90,
        audio_kbps: 128,
        range: Some((t, t + 1.5 / s.fps.max(1.0))),
    };
    let tmp = ffmpeg::scratch_file("still", "mp4", &[]).map_err(|e| e.to_string())?;
    let args = export_args(project, &opts, &tmp)?;
    let run = |cmd: &mut std::process::Command| {
        let o = cmd.stdout(Stdio::null()).stderr(Stdio::piped()).output().map_err(|e| format!("Could not start ffmpeg: {e}"))?;
        if o.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&o.stderr).lines().rev().take(4).collect::<Vec<_>>().join("\n"))
        }
    };
    let result = run(ffmpeg::command("ffmpeg").arg("-y").args(&args))
        .and_then(|_| run(ffmpeg::command("ffmpeg").arg("-y").arg("-i").arg(&tmp).args(["-frames:v", "1"]).arg(out)));
    let _ = std::fs::remove_file(&tmp);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ClipSource, TrackKind};

    fn feed_all(text: &str) -> Progress {
        let mut p = Progress::default();
        text.lines().for_each(|l| p.feed(l));
        p
    }

    #[test]
    fn parses_progress_block() {
        let p = feed_all("frame=30\nout_time_us=1500000\nspeed=2.5x\nprogress=continue");
        assert_eq!(p, Progress { out_secs: 1.5, speed: Some(2.5), ended: false });
    }

    #[test]
    fn parses_end_marker() {
        assert!(feed_all("out_time_us=2000000\nprogress=end").ended);
    }

    #[test]
    fn ignores_unavailable_values() {
        let p = feed_all("out_time_us=N/A\nspeed=N/A\ngarbage");
        assert_eq!(p, Progress::default());
    }

    #[test]
    fn negative_time_clamps_to_zero() {
        assert_eq!(feed_all("out_time_us=-500").out_secs, 0.0);
    }

    fn color_project(secs: f64) -> Project {
        let mut p = Project::default();
        p.settings.width = 320;
        p.settings.height = 180;
        let clip = p.new_clip(ClipSource::Color([200, 40, 40, 255]), "c".into(), 0.0, 0.0, secs);
        let v = p.tracks.iter().position(|t| t.kind == TrackKind::Video).unwrap();
        p.tracks[v].clips.push(clip);
        p
    }

    fn opts() -> ExportOptions {
        ExportOptions { format: Format::Mp4H264, width: 320, height: 180, fps: 30.0, quality: 70, audio_kbps: 128, range: None }
    }

    fn ready() -> bool {
        ffmpeg::available() && export_args(&color_project(1.0), &opts(), Path::new("x.mp4")).is_ok()
    }

    #[test]
    fn rejects_empty_project() {
        let out = std::env::temp_dir().join("cutline-empty.mp4");
        assert!(ExportJob::start(&Project::default(), &opts(), out).is_err());
    }

    #[test]
    fn rejects_missing_output_folder() {
        let out = std::env::temp_dir().join("cutline-no-such-dir").join("a.mp4");
        assert!(ExportJob::start(&color_project(1.0), &opts(), out).is_err());
    }

    #[test]
    fn exports_color_clip_to_done() {
        if !ready() {
            return;
        }
        let out = std::env::temp_dir().join("cutline-job-done.mp4");
        let mut job = ExportJob::start(&color_project(2.0), &opts(), out.clone()).unwrap();
        let t = Instant::now();
        while matches!(job.poll(), JobState::Running { .. }) {
            assert!(t.elapsed().as_secs() < 60);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(matches!(job.poll(), JobState::Done { .. }), "{:?}", job.poll());
        assert!(out.metadata().unwrap().len() > 0);
        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn cancel_stops_and_removes_output() {
        if !ready() {
            return;
        }
        let out = std::env::temp_dir().join("cutline-job-cancel.mp4");
        let mut job = ExportJob::start(&color_project(600.0), &opts(), out.clone()).unwrap();
        job.cancel();
        assert_eq!(job.poll(), &JobState::Cancelled);
        assert!(!out.exists());
    }

    #[test]
    fn exports_still_frame() {
        if !ready() {
            return;
        }
        let out = std::env::temp_dir().join("cutline-still.png");
        export_frame(&color_project(2.0), 0.5, &out).unwrap();
        assert!(out.metadata().unwrap().len() > 0);
        let _ = std::fs::remove_file(out);
    }
}
