use eframe::egui;
use serde_json::Value;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

const API: &str = "https://api.github.com/repos/waykehatescoding/cutline/releases/latest";

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Idle,
    Checking,
    UpToDate,
    Available { version: String, url: String },
    Downloading,
    Failed(String),
}

pub struct Updater {
    ctx: egui::Context,
    status: Arc<Mutex<Status>>,
}

impl Updater {
    pub fn new(ctx: egui::Context) -> Self {
        Self { ctx, status: Arc::new(Mutex::new(Status::Idle)) }
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    pub fn check(&self) {
        if matches!(self.status(), Status::Checking | Status::Downloading) {
            return;
        }
        self.spawn(Status::Checking, || match latest() {
            Ok(Some((version, url))) => Status::Available { version, url },
            Ok(None) => Status::UpToDate,
            Err(e) => Status::Failed(e),
        });
    }

    pub fn install(&self) {
        let Status::Available { url, .. } = self.status() else { return };
        let ctx = self.ctx.clone();
        self.spawn(Status::Downloading, move || match download_and_run(&url) {
            Ok(()) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                Status::Idle
            }
            Err(e) => Status::Failed(e),
        });
    }

    fn spawn(&self, start: Status, job: impl FnOnce() -> Status + Send + 'static) {
        *self.status.lock().unwrap() = start;
        let (status, ctx) = (self.status.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let result = job();
            *status.lock().unwrap() = result;
            ctx.request_repaint();
        });
    }
}

fn curl(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = crate::ffmpeg::command("curl")
        .args(["-fsSL", "--proto", "=https", "-H", "User-Agent: cutline", "-H", "Accept: application/vnd.github+json"])
        .args(args)
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Could not run curl: {e}"))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn latest() -> Result<Option<(String, String)>, String> {
    let body = curl(&[API])?;
    parse_release(&body, env!("CARGO_PKG_VERSION"))
}

fn parse_release(body: &[u8], current: &str) -> Result<Option<(String, String)>, String> {
    let json: Value = serde_json::from_slice(body).map_err(|e| format!("Bad release data: {e}"))?;
    let tag = json["tag_name"].as_str().ok_or("Release has no tag")?;
    if !newer(tag, current) {
        return Ok(None);
    }
    let url = json["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["name"].as_str().is_some_and(|n| n.starts_with("Cutline-Setup") || n == "Cutline.exe"))
        .find_map(|a| a["browser_download_url"].as_str())
        .filter(|u| u.starts_with("https://github.com/"))
        .ok_or("Release has no installer")?;
    Ok(Some((tag.trim_start_matches('v').to_string(), url.to_string())))
}

fn newer(tag: &str, current: &str) -> bool {
    let parse = |v: &str| v.trim_start_matches('v').split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    parse(tag) > parse(current)
}

fn download_and_run(url: &str) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Automatic install is only available on Windows. Download the update from GitHub Releases.".into());
    }
    let path = crate::ffmpeg::scratch_file("cutline-setup", "exe", &[]).map_err(|e| e.to_string())?;
    curl(&["-o", &path.to_string_lossy(), url])?;
    std::process::Command::new(&path)
        .args(["/SILENT", "/SP-", "/SUPPRESSMSGBOXES", "/NOCANCEL", "/CLOSEAPPLICATIONS"])
        .spawn()
        .map_err(|e| format!("Could not start the installer: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(newer("v0.2.0", "0.1.0"));
        assert!(newer("v0.10.0", "0.9.9"));
        assert!(!newer("v0.1.0", "0.1.0"));
        assert!(!newer("v0.0.9", "0.1.0"));
    }

    #[test]
    fn picks_installer_asset() {
        let body = br#"{"tag_name":"v0.2.0","assets":[
            {"name":"cutline-0.2.0-portable.exe","browser_download_url":"https://github.com/a/p.exe"},
            {"name":"Cutline-Setup-0.2.0.exe","browser_download_url":"https://github.com/a/s.exe"}]}"#;
        assert_eq!(parse_release(body, "0.1.0").unwrap(), Some(("0.2.0".into(), "https://github.com/a/s.exe".into())));
        assert_eq!(parse_release(body, "0.2.0").unwrap(), None);
        let evil = br#"{"tag_name":"v9.0.0","assets":[{"name":"Cutline.exe","browser_download_url":"https://evil.test/x.exe"}]}"#;
        assert!(parse_release(evil, "0.1.0").is_err());
    }
}
