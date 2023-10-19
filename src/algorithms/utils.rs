use chess::Action;

#[derive(Debug, Copy, Clone)]
pub(super) struct TranspositionEntry {
    pub(super) depth: u32,
    pub(super) eval: f32,
    pub(super) next_action: Option<Action>,
}

impl TranspositionEntry {
    pub(super) fn new(depth: u32, eval: f32, next_action: Option<Action>) -> Self {
        TranspositionEntry {
            depth,
            eval,
            next_action,
        }
    }
}

pub(super) struct Evaluation {
    pub(super) debug_data: Option<Vec<String>>,
    pub(super) eval: f32,
    pub(super) next_action: Option<Action>,
}

impl Evaluation {
    pub(super) fn new(
        eval: f32,
        next_action: Option<Action>,
        debug_data: Option<Vec<String>>,
    ) -> Evaluation {
        Evaluation {
            eval,
            next_action,
            debug_data,
        }
    }
}
