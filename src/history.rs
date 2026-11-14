use crate::model::Project;

#[derive(Default)]
pub struct History {}

impl History {
    pub fn checkpoint(&mut self, p: &Project) {
        let _ = p;
    }

    pub fn undo(&mut self, p: &mut Project) -> bool {
        let _ = p;
        false
    }

    pub fn redo(&mut self, p: &mut Project) -> bool {
        let _ = p;
        false
    }

    pub fn can_undo(&self) -> bool {
        false
    }

    pub fn can_redo(&self) -> bool {
        false
    }
}
