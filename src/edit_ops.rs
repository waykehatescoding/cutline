use crate::model::*;

const EPS: f64 = 1e-6;
const STILL_LEN: f64 = 5.0;

fn frame(p: &Project) -> f64 {
    1.0 / p.settings.fps.max(1.0)
}

fn media_limit(p: &Project, c: &Clip) -> Option<f64> {
    let m = p.media(c.media_id()?)?;
    (m.kind != MediaKind::Image).then_some(m.duration)
}

fn cut_start(c: &mut Clip, t: f64) {
    let d = (t - c.start) * c.speed;
    if c.reverse {
        c.out_point -= d;
    } else {
        c.in_point += d;
    }
    c.start = t;
}

fn cut_end(c: &mut Clip, t: f64) {
    let d = (t - c.start) * c.speed;
    if c.reverse {
        c.in_point = c.out_point - d;
    } else {
        c.out_point = c.in_point + d;
    }
}

fn settle(t: &mut Track) {
    t.clips.sort_by(|a, b| a.start.total_cmp(&b.start));
    for i in 0..t.clips.len() {
        let prev = i.checked_sub(1).map(|j| (t.clips[j].end(), t.clips[j].duration()));
        let c = &mut t.clips[i];
        let dur = c.duration();
        c.transition_in = match (c.transition_in, prev) {
            (Some(tr), Some((end, pd))) if (end - c.start).abs() < 1e-3 => {
                Some(Transition { duration: tr.duration.min(dur.min(pd) / 2.0), ..tr })
            }
            _ => None,
        };
    }
}

fn cut_range(p: &mut Project, ti: usize, start: f64, end: f64) {
    let clips = std::mem::take(&mut p.tracks[ti].clips);
    let mut kept = Vec::with_capacity(clips.len() + 1);
    for mut c in clips {
        let (cs, ce) = (c.start, c.end());
        if ce <= start + EPS || cs >= end - EPS {
            kept.push(c);
        } else if cs < start - EPS && ce > end + EPS {
            let mut r = c.clone();
            r.id = ClipId(p.alloc_id());
            cut_start(&mut r, end);
            r.transition_in = None;
            r.fade_in = 0.0;
            cut_end(&mut c, start);
            c.fade_out = 0.0;
            kept.push(c);
            kept.push(r);
        } else if cs < start - EPS {
            cut_end(&mut c, start);
            c.fade_out = 0.0;
            kept.push(c);
        } else if ce > end + EPS {
            cut_start(&mut c, end);
            c.transition_in = None;
            c.fade_in = 0.0;
            kept.push(c);
        }
    }
    p.tracks[ti].clips = kept;
}

fn place(p: &mut Project, ti: usize, clip: Clip) -> ClipId {
    let id = clip.id;
    cut_range(p, ti, clip.start, clip.end());
    p.tracks[ti].clips.push(clip);
    settle(&mut p.tracks[ti]);
    id
}

fn video_track(p: &mut Project) -> usize {
    p.tracks.iter().rposition(|t| t.kind == TrackKind::Video && !t.locked).unwrap_or_else(|| add_track(p, TrackKind::Video))
}

fn audio_track(p: &mut Project) -> usize {
    p.tracks.iter().position(|t| t.kind == TrackKind::Audio && !t.locked).unwrap_or_else(|| add_track(p, TrackKind::Audio))
}

fn usable(p: &Project, ti: Option<usize>, kind: TrackKind) -> Option<usize> {
    ti.filter(|&i| p.tracks.get(i).is_some_and(|t| t.kind == kind && !t.locked))
}

pub fn add_media(p: &mut Project, media: MediaId, track: Option<usize>, at: f64) -> Vec<ClipId> {
    let Some(m) = p.media(media) else { return vec![] };
    let (kind, duration, name) = (m.kind, m.duration, m.file_name());
    let want_video = kind == MediaKind::Image || (kind == MediaKind::Video && m.has_video);
    let want_audio = kind == MediaKind::Audio || (kind == MediaKind::Video && m.has_audio);
    let at = at.max(0.0);
    let mut ids = vec![];
    if want_video {
        let len = if kind == MediaKind::Image { STILL_LEN } else { duration };
        let ti = usable(p, track, TrackKind::Video).unwrap_or_else(|| video_track(p));
        let c = p.new_clip(ClipSource::Media(media), name.clone(), at, 0.0, len);
        ids.push(add_clip(p, ti, c));
    }
    if want_audio {
        let ti = usable(p, track.filter(|_| !want_video), TrackKind::Audio).unwrap_or_else(|| audio_track(p));
        let c = p.new_clip(ClipSource::Media(media), name, at, 0.0, duration);
        ids.push(add_clip(p, ti, c));
    }
    ids
}

pub fn add_clip(p: &mut Project, track: usize, clip: Clip) -> ClipId {
    place(p, track, clip)
}

pub fn split_at(p: &mut Project, id: ClipId, t: f64) -> Option<ClipId> {
    let (ti, ci) = p.clip(id)?;
    let c = &p.tracks[ti].clips[ci];
    if p.tracks[ti].locked || t <= c.start + EPS || t >= c.end() - EPS {
        return None;
    }
    let mut r = c.clone();
    r.id = ClipId(p.alloc_id());
    cut_start(&mut r, t);
    r.fade_in = 0.0;
    r.transition_in = None;
    let l = &mut p.tracks[ti].clips[ci];
    cut_end(l, t);
    l.fade_out = 0.0;
    let rid = r.id;
    p.tracks[ti].clips.insert(ci + 1, r);
    Some(rid)
}

pub fn split_all_at(p: &mut Project, t: f64) -> Vec<ClipId> {
    let ids: Vec<ClipId> = p
        .tracks
        .iter()
        .filter(|tr| !tr.locked)
        .flat_map(|tr| tr.clips.iter())
        .filter(|c| t > c.start + EPS && t < c.end() - EPS)
        .map(|c| c.id)
        .collect();
    ids.into_iter().filter_map(|id| split_at(p, id, t)).collect()
}

pub fn delete(p: &mut Project, ids: &[ClipId], ripple: bool) {
    for t in p.tracks.iter_mut().filter(|t| !t.locked) {
        let mut gaps = vec![];
        t.clips.retain(|c| {
            let hit = ids.contains(&c.id);
            if hit {
                gaps.push((c.start, c.duration()));
            }
            !hit
        });
        if ripple {
            gaps.sort_by(|a, b| b.0.total_cmp(&a.0));
            for (start, len) in gaps {
                for c in t.clips.iter_mut().filter(|c| c.start >= start) {
                    c.start -= len;
                }
            }
        }
        settle(t);
    }
}

pub fn move_clip(p: &mut Project, id: ClipId, new_start: f64, new_track: usize) -> bool {
    let Some((ti, ci)) = p.clip(id) else { return false };
    let valid = new_start >= 0.0
        && new_track < p.tracks.len()
        && !p.tracks[ti].locked
        && !p.tracks[new_track].locked
        && p.tracks[ti].kind == p.tracks[new_track].kind;
    if !valid {
        return false;
    }
    let mut c = p.tracks[ti].clips.remove(ci);
    c.start = new_start;
    if ti != new_track {
        settle(&mut p.tracks[ti]);
    }
    place(p, new_track, c);
    true
}

pub fn trim_start(p: &mut Project, id: ClipId, new_start: f64) {
    let Some((ti, ci)) = p.clip(id) else { return };
    if p.tracks[ti].locked {
        return;
    }
    let c = &p.tracks[ti].clips[ci];
    let end = c.end();
    let limit = media_limit(p, c);
    let mut lo = if ci > 0 { p.tracks[ti].clips[ci - 1].end() } else { 0.0 };
    if let Some(d) = limit {
        let room = if c.reverse { d - c.out_point } else { c.in_point };
        lo = lo.max(c.start - room / c.speed);
    }
    let t = new_start.clamp(lo, (end - frame(p)).max(lo));
    let c = &mut p.tracks[ti].clips[ci];
    if limit.is_none() {
        c.start = t;
        c.out_point = c.in_point + (end - t) * c.speed;
    } else {
        cut_start(c, t);
    }
    settle(&mut p.tracks[ti]);
}

pub fn trim_end(p: &mut Project, id: ClipId, new_end: f64) {
    let Some((ti, ci)) = p.clip(id) else { return };
    if p.tracks[ti].locked {
        return;
    }
    let c = &p.tracks[ti].clips[ci];
    let mut hi = p.tracks[ti].clips.get(ci + 1).map_or(f64::INFINITY, |n| n.start);
    if let Some(d) = media_limit(p, c) {
        let room = if c.reverse { c.out_point } else { d - c.in_point };
        hi = hi.min(c.start + room / c.speed);
    }
    let t = new_end.clamp((c.start + frame(p)).min(hi), hi);
    cut_end(&mut p.tracks[ti].clips[ci], t);
    settle(&mut p.tracks[ti]);
}

pub fn slip(p: &mut Project, id: ClipId, delta: f64) {
    let Some((ti, ci)) = p.clip(id) else { return };
    let c = &p.tracks[ti].clips[ci];
    let Some(d) = media_limit(p, c) else { return };
    if p.tracks[ti].locked {
        return;
    }
    let lo = -c.in_point;
    let delta = delta.clamp(lo, (d - c.out_point).max(lo));
    let c = &mut p.tracks[ti].clips[ci];
    c.in_point += delta;
    c.out_point += delta;
}

pub fn duplicate(p: &mut Project, ids: &[ClipId]) -> Vec<ClipId> {
    let sel: Vec<_> = copy(p, ids).into_iter().filter(|(ti, _)| !p.tracks[*ti].locked).collect();
    let end = sel.iter().map(|(_, c)| c.end()).fold(0.0, f64::max);
    paste(p, &sel, end)
}

pub fn copy(p: &Project, ids: &[ClipId]) -> Vec<(usize, Clip)> {
    p.tracks.iter().enumerate().flat_map(|(ti, t)| t.clips.iter().filter(|c| ids.contains(&c.id)).map(move |c| (ti, c.clone()))).collect()
}

fn fits(p: &Project, ti: usize, c: &Clip) -> bool {
    let Some(t) = p.tracks.get(ti) else { return false };
    let kind_ok = match c.media_id().and_then(|m| p.media(m)).map(|m| m.kind) {
        Some(MediaKind::Audio) => t.kind == TrackKind::Audio,
        Some(MediaKind::Video) => true,
        _ => t.kind == TrackKind::Video,
    };
    kind_ok && !t.locked
}

pub fn paste(p: &mut Project, clips: &[(usize, Clip)], at: f64) -> Vec<ClipId> {
    let first = clips.iter().map(|(_, c)| c.start).fold(f64::INFINITY, f64::min);
    let mut ids = vec![];
    for (ti, c) in clips {
        let Some(target) = Some(*ti).filter(|&i| fits(p, i, c)).or_else(|| (0..p.tracks.len()).find(|&i| fits(p, i, c))) else {
            continue;
        };
        let mut c = c.clone();
        c.id = ClipId(p.alloc_id());
        c.start = (at + c.start - first).max(0.0);
        c.transition_in = None;
        ids.push(place(p, target, c));
    }
    ids
}

pub fn snap_points(p: &Project, exclude: &[ClipId], playhead: f64) -> Vec<f64> {
    let mut v = vec![0.0, playhead];
    for c in p.tracks.iter().flat_map(|t| t.clips.iter()).filter(|c| !exclude.contains(&c.id)) {
        v.push(c.start);
        v.push(c.end());
    }
    v.extend(p.markers.iter().map(|m| m.time));
    if let Some((a, b)) = p.range {
        v.extend([a, b]);
    }
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() < EPS);
    v
}

pub fn snap(t: f64, points: &[f64], tolerance: f64) -> Option<f64> {
    points.iter().copied().filter(|x| (x - t).abs() <= tolerance).min_by(|a, b| (a - t).abs().total_cmp(&(b - t).abs()))
}

pub fn close_gaps(p: &mut Project, track: usize) {
    let Some(t) = p.tracks.get_mut(track).filter(|t| !t.locked) else { return };
    t.clips.sort_by(|a, b| a.start.total_cmp(&b.start));
    let mut cursor = 0.0;
    for c in &mut t.clips {
        c.start = cursor;
        cursor = c.end();
    }
    settle(t);
}

pub fn add_track(p: &mut Project, kind: TrackKind) -> usize {
    let n = p.tracks.iter().filter(|t| t.kind == kind).count() + 1;
    let id = TrackId(p.alloc_id());
    let name = match kind {
        TrackKind::Video => format!("V{n}"),
        TrackKind::Audio => format!("A{n}"),
    };
    let track = Track { id, kind, name, muted: false, solo: false, locked: false, hidden: false, volume: 1.0, clips: vec![] };
    if kind == TrackKind::Video {
        p.tracks.insert(0, track);
        0
    } else {
        p.tracks.push(track);
        p.tracks.len() - 1
    }
}

pub fn remove_track(p: &mut Project, index: usize) {
    if index < p.tracks.len() {
        p.tracks.remove(index);
    }
}

pub fn remove_media(p: &mut Project, id: MediaId) {
    p.media.retain(|m| m.id != id);
    for t in &mut p.tracks {
        t.clips.retain(|c| c.media_id() != Some(id));
        settle(t);
    }
}

pub fn set_speed(p: &mut Project, id: ClipId, speed: f64) {
    let Some((ti, ci)) = p.clip(id) else { return };
    if p.tracks[ti].locked {
        return;
    }
    let next = p.tracks[ti].clips.get(ci + 1).map(|n| n.start);
    let c = &mut p.tracks[ti].clips[ci];
    c.speed = speed.clamp(0.1, 8.0);
    if let Some(n) = next.filter(|&n| c.end() > n + EPS) {
        cut_end(c, n);
    }
    settle(&mut p.tracks[ti]);
}

pub fn add_text(p: &mut Project, at: f64) -> Option<ClipId> {
    let ti = p.tracks.iter().position(|t| t.kind == TrackKind::Video && !t.locked).unwrap_or_else(|| add_track(p, TrackKind::Video));
    let style =
        TextStyle { text: "Title".into(), size: 96.0, color: [255, 255, 255, 255], background: None, bold: true, align: TextAlign::Center };
    let c = p.new_clip(ClipSource::Text(style), "Title".into(), at.max(0.0), 0.0, STILL_LEN);
    Some(add_clip(p, ti, c))
}

pub fn add_matte(p: &mut Project, color: [u8; 4], at: f64) -> Option<ClipId> {
    let ti = video_track(p);
    let c = p.new_clip(ClipSource::Color(color), "Matte".into(), at.max(0.0), 0.0, STILL_LEN);
    Some(add_clip(p, ti, c))
}

pub fn set_transition(p: &mut Project, id: ClipId, kind: Option<TransitionKind>, duration: f64) {
    let Some((ti, ci)) = p.clip(id) else { return };
    if p.tracks[ti].locked {
        return;
    }
    let clips = &mut p.tracks[ti].clips;
    let limit =
        (ci > 0 && (clips[ci - 1].end() - clips[ci].start).abs() < 1e-3).then(|| clips[ci - 1].duration().min(clips[ci].duration()) / 2.0);
    clips[ci].transition_in = kind.zip(limit).map(|(kind, l)| Transition { kind, duration: duration.clamp(0.0, l) });
}

pub fn clips_at(p: &Project, t: f64) -> Vec<ClipId> {
    p.tracks.iter().flat_map(|tr| tr.clips.iter()).filter(|c| c.contains(t)).map(|c| c.id).collect()
}

pub fn next_edit(p: &Project, t: f64, forward: bool) -> Option<f64> {
    let edges = p.tracks.iter().flat_map(|tr| tr.clips.iter()).flat_map(|c| [c.start, c.end()]).chain(p.markers.iter().map(|m| m.time));
    if forward {
        edges.filter(|&x| x > t + EPS).min_by(f64::total_cmp)
    } else {
        edges.filter(|&x| x < t - EPS).max_by(f64::total_cmp)
    }
}

pub fn add_marker(p: &mut Project, t: f64) {
    if p.markers.iter().any(|m| (m.time - t).abs() < EPS) {
        return;
    }
    p.markers.push(Marker { time: t, label: String::new(), color: 0 });
    p.markers.sort_by(|a, b| a.time.total_cmp(&b.time));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn add_m(p: &mut Project, kind: MediaKind, duration: f64, has_video: bool, has_audio: bool, name: &str) -> MediaId {
        let id = MediaId(p.alloc_id());
        p.media.push(Media {
            id,
            path: PathBuf::from(name),
            kind,
            duration,
            width: 1920,
            height: 1080,
            fps: 30.0,
            has_video,
            has_audio,
            audio_channels: 2,
            sample_rate: 48000,
            codec: "h264".into(),
        });
        id
    }

    fn setup() -> (Project, MediaId) {
        let mut p = Project::default();
        let m = add_m(&mut p, MediaKind::Video, 60.0, true, true, "clip.mp4");
        (p, m)
    }

    fn put(p: &mut Project, m: MediaId, ti: usize, start: f64, i: f64, o: f64) -> ClipId {
        let c = p.new_clip(ClipSource::Media(m), "c".into(), start, i, o);
        add_clip(p, ti, c)
    }

    fn spans(p: &Project, ti: usize) -> Vec<(f64, f64)> {
        p.tracks[ti].clips.iter().map(|c| (c.start, c.end())).collect()
    }

    fn assert_spans(p: &Project, ti: usize, want: &[(f64, f64)]) {
        let got = spans(p, ti);
        assert_eq!(got.len(), want.len(), "{got:?} vs {want:?}");
        for (g, w) in got.iter().zip(want) {
            assert!(near(g.0, w.0) && near(g.1, w.1), "{got:?} vs {want:?}");
        }
    }

    fn assert_sorted_disjoint(p: &Project) {
        for t in &p.tracks {
            for w in t.clips.windows(2) {
                assert!(w[0].start <= w[1].start && w[0].end() <= w[1].start + 1e-6);
            }
        }
    }

    #[test]
    fn add_media_video_with_audio_adds_both_clips() {
        let (mut p, m) = setup();
        let ids = add_media(&mut p, m, None, 2.0);
        assert_eq!(ids.len(), 2);
        assert_eq!(p.tracks[1].clips.len(), 1);
        assert_eq!(p.tracks[2].clips.len(), 1);
        let v = &p.tracks[1].clips[0];
        assert_eq!(v.id, ids[0]);
        assert!(near(v.start, 2.0) && near(v.in_point, 0.0) && near(v.out_point, 60.0));
        assert_eq!(v.name, "clip.mp4");
        assert_eq!(v.media_id(), Some(m));
        assert_eq!(p.tracks[2].clips[0].id, ids[1]);
        assert!(near(p.tracks[2].clips[0].start, 2.0));
    }

    #[test]
    fn add_media_uses_given_video_track() {
        let (mut p, m) = setup();
        add_media(&mut p, m, Some(0), 0.0);
        assert_eq!(p.tracks[0].clips.len(), 1);
        assert_eq!(p.tracks[1].clips.len(), 0);
    }

    #[test]
    fn add_media_ignores_audio_track_for_video() {
        let (mut p, m) = setup();
        add_media(&mut p, m, Some(3), 0.0);
        assert_eq!(p.tracks[1].clips.len(), 1);
        assert_eq!(p.tracks[2].clips.len(), 1);
        assert_eq!(p.tracks[3].clips.len(), 0);
    }

    #[test]
    fn add_media_video_without_audio_adds_one_clip() {
        let (mut p, _) = setup();
        let m = add_m(&mut p, MediaKind::Video, 10.0, true, false, "mute.mp4");
        assert_eq!(add_media(&mut p, m, None, 0.0).len(), 1);
        assert_eq!(p.tracks[2].clips.len(), 0);
    }

    #[test]
    fn add_media_audio_goes_to_first_audio_track() {
        let (mut p, _) = setup();
        let m = add_m(&mut p, MediaKind::Audio, 30.0, false, true, "song.mp3");
        let ids = add_media(&mut p, m, None, 1.0);
        assert_eq!(ids.len(), 1);
        assert_eq!(p.tracks[2].clips.len(), 1);
        assert!(near(p.tracks[2].clips[0].end(), 31.0));
    }

    #[test]
    fn add_media_audio_uses_given_audio_track() {
        let (mut p, _) = setup();
        let m = add_m(&mut p, MediaKind::Audio, 30.0, false, true, "song.mp3");
        add_media(&mut p, m, Some(3), 0.0);
        assert_eq!(p.tracks[3].clips.len(), 1);
        assert_eq!(p.tracks[2].clips.len(), 0);
    }

    #[test]
    fn add_media_image_is_five_seconds_without_audio() {
        let (mut p, _) = setup();
        let m = add_m(&mut p, MediaKind::Image, 0.0, true, false, "pic.png");
        let ids = add_media(&mut p, m, None, 3.0);
        assert_eq!(ids.len(), 1);
        assert_spans(&p, 1, &[(3.0, 8.0)]);
    }

    #[test]
    fn add_media_skips_locked_tracks() {
        let (mut p, m) = setup();
        p.tracks[1].locked = true;
        p.tracks[2].locked = true;
        add_media(&mut p, m, None, 0.0);
        assert_eq!(p.tracks[0].clips.len(), 1);
        assert_eq!(p.tracks[3].clips.len(), 1);
    }

    #[test]
    fn add_media_creates_missing_tracks() {
        let (mut p, m) = setup();
        p.tracks.clear();
        let ids = add_media(&mut p, m, None, 0.0);
        assert_eq!(ids.len(), 2);
        assert_eq!(p.tracks.len(), 2);
        assert_eq!(p.tracks[0].kind, TrackKind::Video);
        assert_eq!(p.tracks[1].kind, TrackKind::Audio);
        assert_eq!(p.tracks[0].clips.len(), 1);
        assert_eq!(p.tracks[1].clips.len(), 1);
    }

    #[test]
    fn add_media_overwrites_underneath() {
        let (mut p, m) = setup();
        put(&mut p, m, 1, 0.0, 0.0, 10.0);
        let img = add_m(&mut p, MediaKind::Image, 0.0, true, false, "pic.png");
        add_media(&mut p, img, None, 5.0);
        assert_spans(&p, 1, &[(0.0, 5.0), (5.0, 10.0)]);
    }

    #[test]
    fn add_media_unknown_media_adds_nothing() {
        let (mut p, _) = setup();
        assert!(add_media(&mut p, MediaId(999), None, 0.0).is_empty());
    }

    #[test]
    fn split_divides_clip_at_time() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 2.0, 0.0, 10.0);
        let r = split_at(&mut p, id, 6.0).unwrap();
        assert_ne!(r, id);
        assert_spans(&p, 1, &[(2.0, 6.0), (6.0, 12.0)]);
        let l = p.clip_ref(id).unwrap();
        let rc = p.clip_ref(r).unwrap();
        assert!(near(l.out_point, 4.0) && near(l.in_point, 0.0));
        assert!(near(rc.in_point, 4.0) && near(rc.out_point, 10.0));
    }

    #[test]
    fn split_accounts_for_speed() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        p.clip_mut(id).unwrap().speed = 2.0;
        let r = split_at(&mut p, id, 2.0).unwrap();
        let l = p.clip_ref(id).unwrap();
        let rc = p.clip_ref(r).unwrap();
        assert!(near(l.out_point, 4.0));
        assert!(near(rc.in_point, 4.0) && near(rc.out_point, 10.0));
        assert!(near(rc.duration(), 3.0) && near(rc.start, 2.0));
    }

    #[test]
    fn split_accounts_for_reverse() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        p.clip_mut(id).unwrap().reverse = true;
        let r = split_at(&mut p, id, 4.0).unwrap();
        let l = p.clip_ref(id).unwrap();
        let rc = p.clip_ref(r).unwrap();
        assert!(near(l.in_point, 6.0) && near(l.out_point, 10.0));
        assert!(near(rc.in_point, 0.0) && near(rc.out_point, 6.0));
        assert!(rc.reverse && l.reverse);
        assert!(near(l.source_time(4.0), rc.source_time(4.0)));
    }

    #[test]
    fn split_distributes_fades_and_copies_settings() {
        let (mut p, m) = setup();
        put(&mut p, m, 1, 0.0, 0.0, 5.0);
        let id = put(&mut p, m, 1, 5.0, 0.0, 10.0);
        let c = p.clip_mut(id).unwrap();
        c.fade_in = 1.0;
        c.fade_out = 2.0;
        c.transition_in = Some(Transition { kind: TransitionKind::Crossfade, duration: 1.0 });
        c.effects.push(Effect { enabled: true, kind: EffectKind::Grayscale });
        c.transform.x = 0.2;
        let r = split_at(&mut p, id, 10.0).unwrap();
        let l = p.clip_ref(id).unwrap().clone();
        let rc = p.clip_ref(r).unwrap();
        assert!(near(l.fade_in, 1.0) && near(l.fade_out, 0.0));
        assert!(near(rc.fade_out, 2.0) && near(rc.fade_in, 0.0));
        assert!(l.transition_in.is_some() && rc.transition_in.is_none());
        assert_eq!(l.effects, rc.effects);
        assert_eq!(l.transform, rc.transform);
    }

    #[test]
    fn split_rejects_times_not_strictly_inside() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 2.0, 0.0, 10.0);
        assert!(split_at(&mut p, id, 2.0).is_none());
        assert!(split_at(&mut p, id, 12.0).is_none());
        assert!(split_at(&mut p, id, 1.0).is_none());
        assert!(split_at(&mut p, id, 20.0).is_none());
        assert!(split_at(&mut p, ClipId(999), 5.0).is_none());
        assert_spans(&p, 1, &[(2.0, 12.0)]);
    }

    #[test]
    fn split_ignores_locked_track() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        p.tracks[1].locked = true;
        assert!(split_at(&mut p, id, 5.0).is_none());
        assert_eq!(p.tracks[1].clips.len(), 1);
    }

    #[test]
    fn split_all_splits_clips_under_time_on_unlocked_tracks() {
        let (mut p, m) = setup();
        put(&mut p, m, 0, 5.0, 0.0, 10.0);
        put(&mut p, m, 1, 0.0, 0.0, 10.0);
        put(&mut p, m, 2, 0.0, 0.0, 10.0);
        put(&mut p, m, 3, 20.0, 0.0, 5.0);
        p.tracks[2].locked = true;
        let ids = split_all_at(&mut p, 7.0);
        assert_eq!(ids.len(), 2);
        assert_spans(&p, 0, &[(5.0, 7.0), (7.0, 15.0)]);
        assert_spans(&p, 1, &[(0.0, 7.0), (7.0, 10.0)]);
        assert_spans(&p, 2, &[(0.0, 10.0)]);
        assert_spans(&p, 3, &[(20.0, 25.0)]);
    }

    fn three(p: &mut Project, m: MediaId) -> Vec<ClipId> {
        vec![put(p, m, 1, 0.0, 0.0, 5.0), put(p, m, 1, 5.0, 0.0, 5.0), put(p, m, 1, 10.0, 0.0, 5.0)]
    }

    #[test]
    fn delete_removes_clips_leaving_gap() {
        let (mut p, m) = setup();
        let ids = three(&mut p, m);
        delete(&mut p, &[ids[1]], false);
        assert_spans(&p, 1, &[(0.0, 5.0), (10.0, 15.0)]);
    }

    #[test]
    fn delete_ripple_shifts_later_clips() {
        let (mut p, m) = setup();
        let ids = three(&mut p, m);
        delete(&mut p, &[ids[1]], true);
        assert_spans(&p, 1, &[(0.0, 5.0), (5.0, 10.0)]);
    }

    #[test]
    fn delete_ripple_handles_multiple_clips() {
        let (mut p, m) = setup();
        let ids = three(&mut p, m);
        put(&mut p, m, 1, 15.0, 0.0, 5.0);
        delete(&mut p, &[ids[0], ids[2]], true);
        assert_spans(&p, 1, &[(0.0, 5.0), (5.0, 10.0)]);
    }

    #[test]
    fn delete_ripple_keeps_existing_gaps_before_removed_clip() {
        let (mut p, m) = setup();
        put(&mut p, m, 1, 0.0, 0.0, 5.0);
        let b = put(&mut p, m, 1, 8.0, 0.0, 5.0);
        put(&mut p, m, 1, 20.0, 0.0, 5.0);
        delete(&mut p, &[b], true);
        assert_spans(&p, 1, &[(0.0, 5.0), (15.0, 20.0)]);
    }

    #[test]
    fn delete_ripple_leaves_other_tracks_alone() {
        let (mut p, m) = setup();
        let ids = three(&mut p, m);
        put(&mut p, m, 2, 12.0, 0.0, 3.0);
        delete(&mut p, &[ids[0]], true);
        assert_spans(&p, 2, &[(12.0, 15.0)]);
    }

    #[test]
    fn delete_skips_locked_tracks() {
        let (mut p, m) = setup();
        let ids = three(&mut p, m);
        p.tracks[1].locked = true;
        delete(&mut p, &[ids[0]], true);
        assert_eq!(p.tracks[1].clips.len(), 3);
    }

    #[test]
    fn delete_clears_orphaned_transition() {
        let (mut p, m) = setup();
        let ids = three(&mut p, m);
        p.clip_mut(ids[1]).unwrap().transition_in = Some(Transition { kind: TransitionKind::Crossfade, duration: 1.0 });
        delete(&mut p, &[ids[0]], false);
        assert!(p.clip_ref(ids[1]).unwrap().transition_in.is_none());
    }

    #[test]
    fn move_clip_within_track() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 5.0);
        assert!(move_clip(&mut p, id, 10.0, 1));
        assert_spans(&p, 1, &[(10.0, 15.0)]);
    }

    #[test]
    fn move_clip_overwrites_and_trims_neighbours() {
        let (mut p, m) = setup();
        put(&mut p, m, 1, 0.0, 0.0, 10.0);
        let b = put(&mut p, m, 1, 20.0, 0.0, 5.0);
        assert!(move_clip(&mut p, b, 5.0, 1));
        assert_spans(&p, 1, &[(0.0, 5.0), (5.0, 10.0)]);
        assert_sorted_disjoint(&p);
    }

    #[test]
    fn move_clip_splits_clip_underneath() {
        let (mut p, m) = setup();
        put(&mut p, m, 1, 0.0, 0.0, 20.0);
        let b = put(&mut p, m, 0, 100.0, 0.0, 5.0);
        assert!(move_clip(&mut p, b, 5.0, 1));
        assert_spans(&p, 1, &[(0.0, 5.0), (5.0, 10.0), (10.0, 20.0)]);
        assert_eq!(p.tracks[0].clips.len(), 0);
        assert!(near(p.tracks[1].clips[2].in_point, 10.0));
        assert_sorted_disjoint(&p);
    }

    #[test]
    fn move_clip_removes_fully_covered_clips() {
        let (mut p, m) = setup();
        put(&mut p, m, 1, 5.0, 0.0, 2.0);
        let b = put(&mut p, m, 1, 20.0, 0.0, 10.0);
        assert!(move_clip(&mut p, b, 4.0, 1));
        assert_spans(&p, 1, &[(4.0, 14.0)]);
    }

    #[test]
    fn move_clip_rejects_invalid_moves() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 3.0, 0.0, 5.0);
        assert!(!move_clip(&mut p, id, 0.0, 2));
        assert!(!move_clip(&mut p, id, -1.0, 1));
        assert!(!move_clip(&mut p, id, 0.0, 9));
        assert!(!move_clip(&mut p, ClipId(999), 0.0, 1));
        p.tracks[0].locked = true;
        assert!(!move_clip(&mut p, id, 0.0, 0));
        p.tracks[0].locked = false;
        p.tracks[1].locked = true;
        assert!(!move_clip(&mut p, id, 0.0, 0));
        assert_spans(&p, 1, &[(3.0, 8.0)]);
    }

    #[test]
    fn move_clip_between_audio_tracks() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 2, 0.0, 0.0, 5.0);
        assert!(move_clip(&mut p, id, 1.0, 3));
        assert_eq!(p.tracks[2].clips.len(), 0);
        assert_spans(&p, 3, &[(1.0, 6.0)]);
    }

    #[test]
    fn trim_start_shortens_clip() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 5.0, 0.0, 10.0);
        trim_start(&mut p, id, 7.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.start, 7.0) && near(c.in_point, 2.0) && near(c.end(), 15.0));
    }

    #[test]
    fn trim_start_scales_by_speed() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        p.clip_mut(id).unwrap().speed = 2.0;
        trim_start(&mut p, id, 1.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.in_point, 2.0) && near(c.end(), 5.0));
    }

    #[test]
    fn trim_start_extension_stops_at_media_start() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 10.0, 3.0, 8.0);
        trim_start(&mut p, id, 0.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.start, 7.0) && near(c.in_point, 0.0) && near(c.end(), 15.0));
    }

    #[test]
    fn trim_start_extension_stops_at_previous_clip() {
        let (mut p, m) = setup();
        put(&mut p, m, 1, 0.0, 0.0, 8.0);
        let id = put(&mut p, m, 1, 10.0, 20.0, 30.0);
        trim_start(&mut p, id, 2.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.start, 8.0) && near(c.in_point, 18.0) && near(c.end(), 20.0));
    }

    #[test]
    fn trim_start_keeps_one_frame() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        trim_start(&mut p, id, 100.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.duration(), 1.0 / 30.0) && near(c.end(), 10.0));
    }

    #[test]
    fn trim_start_on_reverse_clip_moves_out_point() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 5.0, 0.0, 10.0);
        p.clip_mut(id).unwrap().reverse = true;
        trim_start(&mut p, id, 7.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.out_point, 8.0) && near(c.in_point, 0.0) && near(c.start, 7.0));
        trim_start(&mut p, id, 0.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.start, 0.0) && near(c.out_point, 15.0) && near(c.end(), 15.0));
    }

    #[test]
    fn trim_start_extends_free_sources() {
        let (mut p, _) = setup();
        let id = add_text(&mut p, 3.0).unwrap();
        trim_start(&mut p, id, 1.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.start, 1.0) && near(c.end(), 8.0) && near(c.in_point, 0.0));
        trim_start(&mut p, id, 0.0);
        assert!(near(p.clip_ref(id).unwrap().end(), 8.0));
    }

    #[test]
    fn trim_ignores_locked_track() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        p.tracks[1].locked = true;
        trim_start(&mut p, id, 2.0);
        trim_end(&mut p, id, 5.0);
        assert_spans(&p, 1, &[(0.0, 10.0)]);
    }

    #[test]
    fn trim_end_shortens_clip() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        trim_end(&mut p, id, 6.0);
        assert!(near(p.clip_ref(id).unwrap().out_point, 6.0));
    }

    #[test]
    fn trim_end_extension_stops_at_media_end() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 50.0, 55.0);
        trim_end(&mut p, id, 100.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.out_point, 60.0) && near(c.end(), 10.0));
    }

    #[test]
    fn trim_end_extension_stops_at_next_clip() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 5.0);
        put(&mut p, m, 1, 8.0, 0.0, 5.0);
        trim_end(&mut p, id, 20.0);
        assert!(near(p.clip_ref(id).unwrap().end(), 8.0));
    }

    #[test]
    fn trim_end_keeps_one_frame() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        trim_end(&mut p, id, -5.0);
        assert!(near(p.clip_ref(id).unwrap().duration(), 1.0 / 30.0));
    }

    #[test]
    fn trim_end_on_reverse_clip_moves_in_point() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        p.clip_mut(id).unwrap().reverse = true;
        trim_end(&mut p, id, 6.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.in_point, 4.0) && near(c.out_point, 10.0));
        trim_end(&mut p, id, 100.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.in_point, 0.0) && near(c.end(), 10.0));
    }

    #[test]
    fn trim_end_scales_by_speed() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        p.clip_mut(id).unwrap().speed = 2.0;
        trim_end(&mut p, id, 3.0);
        assert!(near(p.clip_ref(id).unwrap().out_point, 6.0));
    }

    #[test]
    fn trim_end_extends_images_freely() {
        let (mut p, _) = setup();
        let img = add_m(&mut p, MediaKind::Image, 0.0, true, false, "pic.png");
        let ids = add_media(&mut p, img, None, 5.0);
        trim_end(&mut p, ids[0], 30.0);
        assert_spans(&p, 1, &[(5.0, 30.0)]);
    }

    #[test]
    fn trim_end_clears_transition_of_detached_next_clip() {
        let (mut p, m) = setup();
        let a = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        let b = put(&mut p, m, 1, 10.0, 0.0, 10.0);
        set_transition(&mut p, b, Some(TransitionKind::Crossfade), 1.0);
        assert!(p.clip_ref(b).unwrap().transition_in.is_some());
        trim_end(&mut p, a, 6.0);
        assert!(p.clip_ref(b).unwrap().transition_in.is_none());
    }

    #[test]
    fn slip_shifts_in_and_out_together() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 4.0, 10.0, 20.0);
        slip(&mut p, id, 5.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.in_point, 15.0) && near(c.out_point, 25.0) && near(c.start, 4.0));
    }

    #[test]
    fn slip_clamps_to_media_bounds() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 10.0, 20.0);
        slip(&mut p, id, 100.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.in_point, 50.0) && near(c.out_point, 60.0));
        slip(&mut p, id, -100.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.in_point, 0.0) && near(c.out_point, 10.0));
    }

    #[test]
    fn slip_ignores_text_clips() {
        let (mut p, _) = setup();
        let id = add_text(&mut p, 0.0).unwrap();
        slip(&mut p, id, 2.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.in_point, 0.0) && near(c.out_point, 5.0));
    }

    #[test]
    fn duplicate_places_copy_after_original() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 2.0, 7.0);
        let ids = duplicate(&mut p, &[id]);
        assert_eq!(ids.len(), 1);
        assert_ne!(ids[0], id);
        assert_spans(&p, 1, &[(0.0, 5.0), (5.0, 10.0)]);
        let c = p.clip_ref(ids[0]).unwrap();
        assert!(near(c.in_point, 2.0) && near(c.out_point, 7.0));
    }

    #[test]
    fn duplicate_selection_keeps_relative_offsets() {
        let (mut p, m) = setup();
        let a = put(&mut p, m, 1, 0.0, 0.0, 5.0);
        let b = put(&mut p, m, 1, 10.0, 0.0, 5.0);
        let c = put(&mut p, m, 2, 2.0, 0.0, 5.0);
        let ids = duplicate(&mut p, &[a, b, c]);
        assert_eq!(ids.len(), 3);
        assert_spans(&p, 1, &[(0.0, 5.0), (10.0, 15.0), (15.0, 20.0), (25.0, 30.0)]);
        assert_spans(&p, 2, &[(2.0, 7.0), (17.0, 22.0)]);
    }

    #[test]
    fn duplicate_skips_locked_tracks() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 5.0);
        p.tracks[1].locked = true;
        assert!(duplicate(&mut p, &[id]).is_empty());
        assert_eq!(p.tracks[1].clips.len(), 1);
    }

    #[test]
    fn copy_returns_track_index_and_clip() {
        let (mut p, m) = setup();
        let a = put(&mut p, m, 1, 4.0, 0.0, 5.0);
        let b = put(&mut p, m, 2, 5.0, 0.0, 5.0);
        put(&mut p, m, 1, 20.0, 0.0, 5.0);
        let got = copy(&p, &[b, a]);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, 1);
        assert_eq!(got[0].1, *p.clip_ref(a).unwrap());
        assert_eq!(got[1].0, 2);
    }

    #[test]
    fn paste_preserves_relative_offsets() {
        let (mut p, m) = setup();
        let a = put(&mut p, m, 1, 4.0, 0.0, 5.0);
        let b = put(&mut p, m, 2, 5.0, 0.0, 5.0);
        let clips = copy(&p, &[a, b]);
        let ids = paste(&mut p, &clips, 20.0);
        assert_eq!(ids.len(), 2);
        assert!(!ids.contains(&a) && !ids.contains(&b));
        assert_spans(&p, 1, &[(4.0, 9.0), (20.0, 25.0)]);
        assert_spans(&p, 2, &[(5.0, 10.0), (21.0, 26.0)]);
    }

    #[test]
    fn paste_overwrites_existing_clips() {
        let (mut p, m) = setup();
        let a = put(&mut p, m, 1, 0.0, 0.0, 5.0);
        put(&mut p, m, 1, 10.0, 0.0, 10.0);
        let clips = copy(&p, &[a]);
        paste(&mut p, &clips, 12.0);
        assert_spans(&p, 1, &[(0.0, 5.0), (10.0, 12.0), (12.0, 17.0), (17.0, 20.0)]);
        assert_sorted_disjoint(&p);
    }

    #[test]
    fn paste_falls_back_to_compatible_track() {
        let (mut p, _) = setup();
        let am = add_m(&mut p, MediaKind::Audio, 30.0, false, true, "song.mp3");
        let c = p.new_clip(ClipSource::Media(am), "s".into(), 0.0, 0.0, 4.0);
        paste(&mut p, &[(9, c.clone())], 3.0);
        assert_spans(&p, 2, &[(3.0, 7.0)]);
        p.tracks[2].locked = true;
        paste(&mut p, &[(2, c)], 0.0);
        assert_spans(&p, 3, &[(0.0, 4.0)]);
    }

    #[test]
    fn paste_empty_does_nothing() {
        let (mut p, _) = setup();
        assert!(paste(&mut p, &[], 5.0).is_empty());
    }

    #[test]
    fn snap_points_collects_all_targets() {
        let (mut p, m) = setup();
        let a = put(&mut p, m, 1, 2.0, 0.0, 3.0);
        put(&mut p, m, 2, 10.0, 0.0, 4.0);
        add_marker(&mut p, 7.5);
        p.range = Some((1.0, 20.0));
        let pts = snap_points(&p, &[], 6.0);
        for want in [0.0, 1.0, 2.0, 5.0, 6.0, 7.5, 10.0, 14.0, 20.0] {
            assert!(pts.iter().any(|x| near(*x, want)), "{want} missing in {pts:?}");
        }
        assert!(pts.windows(2).all(|w| w[0] < w[1]));
        let pts = snap_points(&p, &[a], 6.0);
        assert!(!pts.iter().any(|x| near(*x, 2.0) || near(*x, 5.0)));
    }

    #[test]
    fn snap_picks_nearest_within_tolerance() {
        let pts = [0.0, 4.0, 5.0, 10.0];
        assert_eq!(snap(4.6, &pts, 1.0), Some(5.0));
        assert_eq!(snap(4.4, &pts, 1.0), Some(4.0));
        assert_eq!(snap(7.0, &pts, 1.0), None);
        assert_eq!(snap(5.0, &pts, 0.0), Some(5.0));
        assert_eq!(snap(1.0, &[], 1.0), None);
    }

    #[test]
    fn close_gaps_pulls_clips_left() {
        let (mut p, m) = setup();
        put(&mut p, m, 1, 3.0, 0.0, 2.0);
        put(&mut p, m, 1, 8.0, 0.0, 2.0);
        put(&mut p, m, 1, 10.0, 0.0, 2.0);
        close_gaps(&mut p, 1);
        assert_spans(&p, 1, &[(0.0, 2.0), (2.0, 4.0), (4.0, 6.0)]);
    }

    #[test]
    fn close_gaps_skips_locked_track_and_bad_index() {
        let (mut p, m) = setup();
        put(&mut p, m, 1, 3.0, 0.0, 2.0);
        p.tracks[1].locked = true;
        close_gaps(&mut p, 1);
        close_gaps(&mut p, 50);
        assert_spans(&p, 1, &[(3.0, 5.0)]);
    }

    #[test]
    fn add_track_video_goes_on_top() {
        let mut p = Project::default();
        let i = add_track(&mut p, TrackKind::Video);
        assert_eq!(i, 0);
        assert_eq!(p.tracks.len(), 5);
        assert_eq!(p.tracks[0].name, "V3");
        assert_eq!(p.tracks[0].kind, TrackKind::Video);
        assert_eq!(p.tracks[1].name, "V2");
    }

    #[test]
    fn add_track_audio_is_appended() {
        let mut p = Project::default();
        let i = add_track(&mut p, TrackKind::Audio);
        assert_eq!(i, 4);
        assert_eq!(p.tracks[4].name, "A3");
        assert_eq!(p.tracks[4].kind, TrackKind::Audio);
        let ids: std::collections::HashSet<_> = p.tracks.iter().map(|t| t.id).collect();
        assert_eq!(ids.len(), 5);
    }

    #[test]
    fn remove_track_removes_by_index() {
        let mut p = Project::default();
        remove_track(&mut p, 1);
        assert_eq!(p.tracks.len(), 3);
        assert_eq!(p.tracks[1].name, "A1");
        remove_track(&mut p, 50);
        assert_eq!(p.tracks.len(), 3);
    }

    #[test]
    fn remove_media_removes_its_clips_everywhere() {
        let (mut p, m) = setup();
        let other = add_m(&mut p, MediaKind::Video, 10.0, true, true, "other.mp4");
        add_media(&mut p, m, None, 0.0);
        let keep = put(&mut p, other, 0, 0.0, 0.0, 5.0);
        add_text(&mut p, 50.0);
        remove_media(&mut p, m);
        assert!(p.media(m).is_none() && p.media(other).is_some());
        assert_eq!(p.tracks[1].clips.len(), 0);
        assert_eq!(p.tracks[2].clips.len(), 0);
        assert!(p.clip_ref(keep).is_some());
        assert_eq!(p.tracks.iter().map(|t| t.clips.len()).sum::<usize>(), 2);
    }

    #[test]
    fn set_speed_keeps_start_and_changes_duration() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 4.0, 0.0, 10.0);
        set_speed(&mut p, id, 2.0);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.start, 4.0) && near(c.duration(), 5.0) && near(c.end(), 9.0));
    }

    #[test]
    fn set_speed_clamps_range() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        set_speed(&mut p, id, 100.0);
        assert!(near(p.clip_ref(id).unwrap().speed, 8.0));
        set_speed(&mut p, id, 0.0);
        assert!(near(p.clip_ref(id).unwrap().speed, 0.1));
    }

    #[test]
    fn set_speed_trims_out_to_fit_next_clip() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        put(&mut p, m, 1, 12.0, 0.0, 5.0);
        set_speed(&mut p, id, 0.5);
        let c = p.clip_ref(id).unwrap();
        assert!(near(c.end(), 12.0) && near(c.out_point, 6.0) && near(c.speed, 0.5));
        assert_sorted_disjoint(&p);
    }

    #[test]
    fn set_speed_ignores_locked_track() {
        let (mut p, m) = setup();
        let id = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        p.tracks[1].locked = true;
        set_speed(&mut p, id, 2.0);
        assert!(near(p.clip_ref(id).unwrap().speed, 1.0));
    }

    #[test]
    fn add_text_uses_top_video_track() {
        let (mut p, _) = setup();
        let id = add_text(&mut p, 2.0).unwrap();
        assert_eq!(p.clip(id), Some((0, 0)));
        let c = p.clip_ref(id).unwrap();
        assert_eq!(c.name, "Title");
        assert!(near(c.start, 2.0) && near(c.duration(), 5.0));
        assert_eq!(
            c.source,
            ClipSource::Text(TextStyle {
                text: "Title".into(),
                size: 96.0,
                color: [255, 255, 255, 255],
                background: None,
                bold: true,
                align: TextAlign::Center
            })
        );
    }

    #[test]
    fn add_text_skips_locked_top_track() {
        let (mut p, _) = setup();
        p.tracks[0].locked = true;
        let id = add_text(&mut p, 0.0).unwrap();
        assert_eq!(p.clip(id).unwrap().0, 1);
    }

    #[test]
    fn add_text_creates_video_track_when_missing() {
        let (mut p, _) = setup();
        p.tracks.retain(|t| t.kind == TrackKind::Audio);
        let id = add_text(&mut p, 0.0).unwrap();
        assert_eq!(p.clip(id).unwrap().0, 0);
        assert_eq!(p.tracks[0].kind, TrackKind::Video);
    }

    #[test]
    fn add_matte_uses_bottom_video_track() {
        let (mut p, _) = setup();
        let id = add_matte(&mut p, [10, 20, 30, 255], 1.0).unwrap();
        assert_eq!(p.clip(id).unwrap().0, 1);
        let c = p.clip_ref(id).unwrap();
        assert_eq!(c.source, ClipSource::Color([10, 20, 30, 255]));
        assert!(near(c.start, 1.0) && near(c.duration(), 5.0));
    }

    #[test]
    fn add_matte_skips_locked_track() {
        let (mut p, _) = setup();
        p.tracks[1].locked = true;
        let id = add_matte(&mut p, [0, 0, 0, 255], 0.0).unwrap();
        assert_eq!(p.clip(id).unwrap().0, 0);
    }

    #[test]
    fn set_transition_needs_adjacent_previous_clip() {
        let (mut p, m) = setup();
        let a = put(&mut p, m, 1, 0.0, 0.0, 10.0);
        let b = put(&mut p, m, 1, 20.0, 0.0, 10.0);
        set_transition(&mut p, a, Some(TransitionKind::Crossfade), 1.0);
        set_transition(&mut p, b, Some(TransitionKind::Crossfade), 1.0);
        assert!(p.clip_ref(a).unwrap().transition_in.is_none());
        assert!(p.clip_ref(b).unwrap().transition_in.is_none());
    }

    #[test]
    fn set_transition_applies_and_removes() {
        let (mut p, m) = setup();
        put(&mut p, m, 1, 0.0, 0.0, 10.0);
        let b = put(&mut p, m, 1, 10.0, 0.0, 10.0);
        set_transition(&mut p, b, Some(TransitionKind::WipeLeft), 1.5);
        let t = p.clip_ref(b).unwrap().transition_in.unwrap();
        assert_eq!(t.kind, TransitionKind::WipeLeft);
        assert!(near(t.duration, 1.5));
        set_transition(&mut p, b, None, 1.0);
        assert!(p.clip_ref(b).unwrap().transition_in.is_none());
    }

    #[test]
    fn set_transition_clamps_to_half_of_shorter_clip() {
        let (mut p, m) = setup();
        put(&mut p, m, 1, 0.0, 0.0, 4.0);
        let b = put(&mut p, m, 1, 4.0, 0.0, 10.0);
        set_transition(&mut p, b, Some(TransitionKind::Crossfade), 100.0);
        assert!(near(p.clip_ref(b).unwrap().transition_in.unwrap().duration, 2.0));
    }

    #[test]
    fn clips_at_returns_clips_under_time_in_track_order() {
        let (mut p, m) = setup();
        let a = put(&mut p, m, 2, 0.0, 0.0, 10.0);
        let b = put(&mut p, m, 1, 5.0, 0.0, 10.0);
        assert_eq!(clips_at(&p, 7.0), vec![b, a]);
        assert_eq!(clips_at(&p, 10.0), vec![b]);
        assert!(clips_at(&p, 20.0).is_empty());
    }

    #[test]
    fn next_edit_walks_boundaries_and_markers() {
        let (mut p, m) = setup();
        put(&mut p, m, 1, 2.0, 0.0, 3.0);
        put(&mut p, m, 2, 4.0, 0.0, 4.0);
        add_marker(&mut p, 6.5);
        let fwd: Vec<f64> = std::iter::successors(next_edit(&p, 0.0, true), |t| next_edit(&p, *t, true)).collect();
        assert_eq!(fwd, vec![2.0, 4.0, 5.0, 6.5, 8.0]);
        let back: Vec<f64> = std::iter::successors(next_edit(&p, 9.0, false), |t| next_edit(&p, *t, false)).collect();
        assert_eq!(back, vec![8.0, 6.5, 5.0, 4.0, 2.0]);
        assert_eq!(next_edit(&p, 8.0, true), None);
        assert_eq!(next_edit(&p, 2.0, false), None);
    }

    #[test]
    fn add_marker_ignores_duplicates_and_keeps_order() {
        let mut p = Project::default();
        add_marker(&mut p, 5.0);
        add_marker(&mut p, 2.0);
        add_marker(&mut p, 5.0);
        let times: Vec<f64> = p.markers.iter().map(|m| m.time).collect();
        assert_eq!(times, vec![2.0, 5.0]);
    }
}
