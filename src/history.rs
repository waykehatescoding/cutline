use crate::model::Project;

const CAP: usize = 200;

struct Snapshot {
    project: Project,
    json: Vec<u8>,
}

impl Snapshot {
    fn new(p: &Project) -> Self {
        Self { project: p.clone(), json: serde_json::to_vec(p).unwrap_or_default() }
    }
}

#[derive(Default)]
pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

impl History {
    pub fn checkpoint(&mut self, p: &Project) {
        let snap = Snapshot::new(p);
        if self.undo.last().is_some_and(|s| s.json == snap.json) {
            return;
        }
        self.redo.clear();
        self.undo.push(snap);
        if self.undo.len() > CAP {
            self.undo.remove(0);
        }
    }

    pub fn undo(&mut self, p: &mut Project) -> bool {
        let Some(prev) = self.undo.pop() else { return false };
        self.redo.push(Snapshot::new(p));
        *p = prev.project;
        true
    }

    pub fn redo(&mut self, p: &mut Project) -> bool {
        let Some(next) = self.redo.pop() else { return false };
        self.undo.push(Snapshot::new(p));
        *p = next.project;
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(n: &str) -> Project {
        Project { name: n.into(), ..Project::default() }
    }

    #[test]
    fn undo_redo_sequence() {
        let mut h = History::default();
        let mut p = named("a");
        h.checkpoint(&p);
        p.name = "b".into();
        h.checkpoint(&p);
        p.name = "c".into();
        assert!(h.undo(&mut p));
        assert_eq!(p.name, "b");
        assert!(h.undo(&mut p));
        assert_eq!(p.name, "a");
        assert!(!h.undo(&mut p));
        assert!(h.redo(&mut p));
        assert_eq!(p.name, "b");
        assert!(h.redo(&mut p));
        assert_eq!(p.name, "c");
        assert!(!h.redo(&mut p));
    }

    #[test]
    fn checkpoint_clears_redo() {
        let mut h = History::default();
        let mut p = named("a");
        h.checkpoint(&p);
        p.name = "b".into();
        h.undo(&mut p);
        assert!(h.can_redo());
        p.name = "x".into();
        h.checkpoint(&p);
        assert!(!h.can_redo());
    }

    #[test]
    fn identical_checkpoint_skipped() {
        let mut h = History::default();
        let p = named("a");
        h.checkpoint(&p);
        h.checkpoint(&p);
        let mut q = p.clone();
        assert!(h.undo(&mut q));
        assert!(!h.can_undo());
    }

    #[test]
    fn capped_at_200() {
        let mut h = History::default();
        let mut p = named("0");
        for i in 0..250 {
            p.name = i.to_string();
            h.checkpoint(&p);
        }
        let mut n = 0;
        while h.undo(&mut p) {
            n += 1;
        }
        assert_eq!(n, 200);
        assert_eq!(p.name, "50");
    }
}
