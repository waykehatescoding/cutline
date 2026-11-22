const VIDEO: [&str; 9] = ["mp4", "mov", "mkv", "webm", "avi", "m4v", "wmv", "flv", "ts"];
const AUDIO: [&str; 4] = ["mp3", "wav", "flac", "m4a"];

const EDIT: &str = "Cutline";
const PLAY: &str = "CutlinePlay";

fn key_path(ext: &str, verb: &str) -> String {
    format!(r"HKCU\Software\Classes\SystemFileAssociations\.{ext}\shell\{verb}")
}

fn open_command(exe: &str) -> String {
    format!("\"{exe}\" \"%1\"")
}

fn play_command(exe: &str) -> String {
    format!("\"{exe}\" --play \"%1\"")
}

fn label(ext: &str) -> &'static str {
    if AUDIO.contains(&ext) {
        "Edit with Cutline"
    } else {
        "Edit this video"
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Output};

    fn reg(args: &[&str]) -> Result<Output, String> {
        Command::new("reg").args(args).creation_flags(0x08000000).output().map_err(|e| format!("Could not run reg.exe: {e}"))
    }

    fn reg_ok(args: &[&str]) -> Result<(), String> {
        let out = reg(args)?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }

    fn exe() -> Result<String, String> {
        std::env::current_exe().map(|p| p.display().to_string()).map_err(|e| e.to_string())
    }

    pub fn is_registered() -> bool {
        let Ok(exe) = exe() else { return false };
        let key = format!(r"{}\command", key_path("mp4", PLAY));
        reg(&["query", &key, "/ve"]).is_ok_and(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains(&exe))
    }

    pub fn register() -> Result<(), String> {
        let exe = exe()?;
        let (edit, play) = (open_command(&exe), play_command(&exe));
        let icon = format!("{exe},0");
        let add = |key: String, name: &str, command: &str| {
            let sub = format!(r"{key}\command");
            reg_ok(&["add", &key, "/v", "MUIVerb", "/d", name, "/f"])?;
            reg_ok(&["add", &key, "/v", "Icon", "/d", &icon, "/f"])?;
            reg_ok(&["add", &sub, "/ve", "/d", command, "/f"])
        };
        for ext in VIDEO {
            add(key_path(ext, PLAY), "Play with Cutline", &play)?;
        }
        for ext in VIDEO.iter().chain(AUDIO.iter()) {
            add(key_path(ext, EDIT), label(ext), &edit)?;
        }
        Ok(())
    }

    pub fn unregister() -> Result<(), String> {
        for ext in VIDEO.iter().chain(AUDIO.iter()) {
            for verb in [EDIT, PLAY] {
                let out = reg(&["delete", &key_path(ext, verb), "/f"])?;
                let err = String::from_utf8_lossy(&out.stderr);
                if !out.status.success() && !err.contains("unable to find") {
                    return Err(err.trim().to_string());
                }
            }
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn is_registered() -> bool {
        false
    }

    pub fn register() -> Result<(), String> {
        Err("Explorer integration is only available on Windows".into())
    }

    pub fn unregister() -> Result<(), String> {
        Err("Explorer integration is only available on Windows".into())
    }
}

pub use imp::{is_registered, register, unregister};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_key_path() {
        assert_eq!(key_path("mp4", EDIT), r"HKCU\Software\Classes\SystemFileAssociations\.mp4\shell\Cutline");
        assert_eq!(key_path("mp4", PLAY), r"HKCU\Software\Classes\SystemFileAssociations\.mp4\shell\CutlinePlay");
    }

    #[test]
    fn builds_play_command() {
        assert_eq!(play_command(r"C:\Apps\cutline.exe"), r#""C:\Apps\cutline.exe" --play "%1""#);
    }

    #[test]
    fn builds_open_command() {
        assert_eq!(open_command(r"C:\Apps\cutline.exe"), r#""C:\Apps\cutline.exe" "%1""#);
    }

    #[test]
    fn labels_audio_and_video() {
        assert_eq!(label("mp4"), "Edit this video");
        assert_eq!(label("wav"), "Edit with Cutline");
    }
}
