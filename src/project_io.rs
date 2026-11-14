use crate::model::Project;
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};

const VERSION: u64 = 1;
const MAX_RECENT: usize = 10;

fn relative_to(path: &Path, base: &Path) -> Option<PathBuf> {
    let p: Vec<_> = path.components().collect();
    let b: Vec<_> = base.components().collect();
    let common = p.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == 0 || matches!(p[0], Component::Prefix(_)) && common < 2 {
        return None;
    }
    let mut out = PathBuf::new();
    for _ in common..b.len() {
        out.push("..");
    }
    out.extend(&p[common..]);
    Some(out)
}

pub fn save(p: &Project, path: &Path) -> Result<(), String> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let rel: Vec<Option<String>> = p.media.iter().map(|m| relative_to(&m.path, dir).map(|r| r.to_string_lossy().into_owned())).collect();
    let doc = json!({ "app": "cutline", "version": VERSION, "project": p, "media_relative": rel });
    let bytes = serde_json::to_vec_pretty(&doc).map_err(|e| e.to_string())?;
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

pub fn load(path: &Path) -> Result<Project, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Cannot open project: {e}"))?;
    let not_project = || "Not a Cutline project".to_string();
    let mut doc: Value = serde_json::from_slice(&bytes).map_err(|_| not_project())?;
    if doc["app"] != "cutline" {
        return Err(not_project());
    }
    let version = doc["version"].as_u64().ok_or_else(not_project)?;
    if version > VERSION {
        return Err("Project was saved by a newer version".into());
    }
    let rel = doc["media_relative"].take();
    let mut p: Project = serde_json::from_value(doc["project"].take()).map_err(|e| format!("Project file is damaged: {e}"))?;
    let dir = path.parent().unwrap_or(Path::new("."));
    for (i, m) in p.media.iter_mut().enumerate() {
        if m.path.exists() {
            continue;
        }
        if let Some(r) = rel[i].as_str() {
            let candidate = dir.join(r);
            if candidate.exists() {
                m.path = candidate;
            }
        }
    }
    let max = p
        .media
        .iter()
        .map(|m| m.id.0)
        .chain(p.tracks.iter().flat_map(|t| t.clips.iter().map(|c| c.id.0).chain([t.id.0])))
        .max()
        .unwrap_or(0);
    p.next_id = p.next_id.max(max + 1);
    Ok(p)
}

fn config_dir() -> PathBuf {
    let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let base = if cfg!(windows) { var("APPDATA") } else { var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|h| h.join(".config"))) };
    base.unwrap_or_else(std::env::temp_dir).join("cutline")
}

pub fn autosave_path() -> PathBuf {
    config_dir().join("autosave.cutline")
}

fn recent_from(file: &Path) -> Vec<PathBuf> {
    let list: Vec<PathBuf> = std::fs::read(file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    list.into_iter().filter(|p| p.exists()).take(MAX_RECENT).collect()
}

fn push_recent_to(file: &Path, path: &Path) {
    let mut list = recent_from(file);
    list.retain(|p| p != path);
    list.insert(0, path.to_path_buf());
    list.truncate(MAX_RECENT);
    if let (Some(dir), Ok(bytes)) = (file.parent(), serde_json::to_vec(&list)) {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(file, bytes);
    }
}

pub fn recent() -> Vec<PathBuf> {
    recent_from(&config_dir().join("recent.json"))
}

pub fn push_recent(path: &Path) {
    push_recent_to(&config_dir().join("recent.json"), path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ClipSource, Media, MediaId, MediaKind};
    use std::fs;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cutline-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn media(p: &mut Project, path: PathBuf) -> MediaId {
        let id = MediaId(p.alloc_id());
        p.media.push(Media {
            id,
            path,
            kind: MediaKind::Video,
            duration: 10.0,
            width: 1920,
            height: 1080,
            fps: 30.0,
            has_video: true,
            has_audio: false,
            audio_channels: 0,
            sample_rate: 0,
            codec: "h264".into(),
        });
        id
    }

    #[test]
    fn round_trip() {
        let d = temp("round");
        let mut p = Project::default();
        p.name = "Demo".into();
        let m = media(&mut p, d.join("a.mp4"));
        let c = p.new_clip(ClipSource::Media(m), "a".into(), 1.0, 0.0, 5.0);
        p.tracks[1].clips.push(c);
        let file = d.join("demo.cutline");
        save(&p, &file).unwrap();
        assert!(!d.join("demo.cutline.tmp").exists());
        let q = load(&file).unwrap();
        assert_eq!(serde_json::to_vec(&p).unwrap(), serde_json::to_vec(&q).unwrap());
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"app\": \"cutline\""));
        assert!(text.contains("\"version\": 1"));
    }

    #[test]
    fn relative_path_fallback() {
        let d = temp("rel");
        fs::create_dir_all(d.join("media")).unwrap();
        fs::write(d.join("media").join("a.mp4"), b"x").unwrap();
        let mut p = Project::default();
        media(&mut p, d.join("media").join("a.mp4"));
        let file = d.join("demo.cutline");
        save(&p, &file).unwrap();
        let moved = temp("rel-moved");
        fs::create_dir_all(moved.join("media")).unwrap();
        fs::write(moved.join("media").join("a.mp4"), b"x").unwrap();
        fs::copy(&file, moved.join("demo.cutline")).unwrap();
        fs::remove_file(d.join("media").join("a.mp4")).unwrap();
        let q = load(&moved.join("demo.cutline")).unwrap();
        assert_eq!(q.media[0].path, moved.join("media").join("a.mp4"));
    }

    #[test]
    fn existing_absolute_path_wins() {
        let d = temp("abs");
        fs::write(d.join("a.mp4"), b"x").unwrap();
        let mut p = Project::default();
        media(&mut p, d.join("a.mp4"));
        let file = d.join("demo.cutline");
        save(&p, &file).unwrap();
        assert_eq!(load(&file).unwrap().media[0].path, d.join("a.mp4"));
    }

    #[test]
    fn next_id_exceeds_all_ids() {
        let d = temp("ids");
        let mut p = Project::default();
        let m = media(&mut p, d.join("a.mp4"));
        let c = p.new_clip(ClipSource::Media(m), "a".into(), 0.0, 0.0, 1.0);
        p.tracks[0].clips.push(c);
        p.next_id = 1;
        let file = d.join("demo.cutline");
        save(&p, &file).unwrap();
        let q = load(&file).unwrap();
        let max = q
            .tracks
            .iter()
            .flat_map(|t| t.clips.iter().map(|c| c.id.0).chain([t.id.0]))
            .chain(q.media.iter().map(|m| m.id.0))
            .max()
            .unwrap();
        assert!(q.next_id > max);
    }

    #[test]
    fn rejects_garbage() {
        let d = temp("bad");
        let file = d.join("x.cutline");
        fs::write(&file, b"nope").unwrap();
        assert_eq!(load(&file).unwrap_err(), "Not a Cutline project");
        fs::write(&file, br#"{"app":"other","version":1,"project":{}}"#).unwrap();
        assert_eq!(load(&file).unwrap_err(), "Not a Cutline project");
    }

    #[test]
    fn rejects_newer_version() {
        let d = temp("newer");
        let file = d.join("x.cutline");
        fs::write(&file, br#"{"app":"cutline","version":2,"project":{}}"#).unwrap();
        assert_eq!(load(&file).unwrap_err(), "Project was saved by a newer version");
    }

    #[test]
    fn missing_file_errors() {
        let d = temp("missing");
        assert!(load(&d.join("none.cutline")).is_err());
    }

    #[test]
    fn recent_front_dedupe_cap_and_filter() {
        let d = temp("recent");
        let file = d.join("recent.json");
        let files: Vec<PathBuf> = (0..12)
            .map(|i| {
                let f = d.join(format!("p{i}.cutline"));
                fs::write(&f, b"x").unwrap();
                f
            })
            .collect();
        for f in &files {
            push_recent_to(&file, f);
        }
        push_recent_to(&file, &files[5]);
        let r = recent_from(&file);
        assert_eq!(r.len(), 10);
        assert_eq!(r[0], files[5]);
        assert_eq!(r.iter().filter(|p| **p == files[5]).count(), 1);
        fs::remove_file(&files[11]).unwrap();
        assert!(!recent_from(&file).contains(&files[11]));
    }
}
