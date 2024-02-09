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

#[derive(Debug)]
pub(super) struct Evaluation {
    pub(super) debug_data: Option<Vec<String>>,
    pub(super) eval: Option<f32>,
    pub(super) next_action: Option<Action>,
    pub(super) white_incremental_psqt_eval: Option<f32>,
    pub(super) black_incremental_psqt_eval: Option<f32>,
}

impl Evaluation {
    pub(super) fn new(
        eval: Option<f32>,
        next_action: Option<Action>,
        debug_data: Option<Vec<String>>,
        white_incremental_psqt_eval: Option<f32>,
        black_incremental_psqt_eval: Option<f32>,
    ) -> Evaluation {
        Evaluation {
            eval,
            next_action,
            debug_data,
            white_incremental_psqt_eval,
            black_incremental_psqt_eval
        }
    }
}
