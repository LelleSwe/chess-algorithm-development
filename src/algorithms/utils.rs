use chess::Action;

#[derive(Debug, Clone, PartialEq, Copy)]
pub(crate) struct Evaluation {
    // pub(super) debug_data: Option<Vec<String>>,
    pub(crate) eval: Option<f32>,
    pub(crate) next_action: Option<Action>,
    pub(super) incremental_psqt_eval: Option<f32>,
}

impl Evaluation {
    pub(crate) fn new(
        eval: Option<f32>,
        next_action: Option<Action>,
        // debug_data: Option<Vec<String>>,
        incremental_psqt_eval: Option<f32>,
    ) -> Evaluation {
        Evaluation {
            eval,
            next_action,
            // debug_data,
            incremental_psqt_eval,
        }
    }

    pub fn empty() -> Evaluation {
        Evaluation {
            eval: None,
            next_action: None,
            // debug_data: None,
            incremental_psqt_eval: None,
        }
    }
}
