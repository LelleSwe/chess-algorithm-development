use chess::{Action, ChessMove};

use crate::algorithms::utils::Evaluation;
use crate::common::utils;

#[must_use]
pub(crate) fn get_debug_data(
    modules: u32,
    maximise: bool,
    best_evaluation: &Evaluation,
    chess_move: &ChessMove,
    evaluation: &Evaluation,
) -> Vec<String> {
    let mut vec = Vec::new();
    let new_best_move = chess_move.to_string();
    let new_best_eval = evaluation.eval;
    utils::vector_push_debug!(
        vec,
        modules,
        maximise,
        best_evaluation.eval,
        new_best_move,
        new_best_eval,
    );
    if let Some(Action::MakeMove(previous_best_move)) = best_evaluation.next_action {
        let previous_best_move = previous_best_move.to_string();
        utils::vector_push_debug!(vec, previous_best_move);
    }
    return vec;
}
