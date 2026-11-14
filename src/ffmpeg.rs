use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

pub fn bin(name: &str) -> PathBuf {
    let file = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join(&file)))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from(file))
}

pub fn command(name: &str) -> Command {
    let mut c = Command::new(bin(name));
    c.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c
}

pub const SAFE_INPUT: [&str; 2] = ["-protocol_whitelist", "file"];

pub fn scratch_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("XDG_CACHE_HOME"))
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
            .unwrap_or_else(std::env::temp_dir);
        let dir = base.join("cutline").join("tmp");
        let _ = std::fs::create_dir_all(&dir);
        let day = Duration::from_secs(86_400);
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            if entry.metadata().and_then(|m| m.modified()).is_ok_and(|t| t.elapsed().is_ok_and(|e| e > day)) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        dir
    })
    .clone()
}

pub fn scratch_file(stem: &str, ext: &str, contents: &[u8]) -> std::io::Result<PathBuf> {
    static N: AtomicUsize = AtomicUsize::new(0);
    loop {
        let path = scratch_dir().join(format!("{stem}-{}-{}.{ext}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => return f.write_all(contents).map(|_| path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
}

pub fn available() -> bool {
    ["ffmpeg", "ffprobe"]
        .iter()
        .all(|n| command(n).arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false))
}
