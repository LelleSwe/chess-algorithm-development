use crate::algorithms::utils::Evaluation;
use chess::{Board, Color};

/// Investigate the reason for there being no legal moves, and return a score based on that.
pub fn eval_no_legal_moves(board: &Board) -> f32 {
    if board.checkers().popcnt() == 0 {
        // Is Stalemate, no checking pieces
        return 0.;
    }

    // If we arrive at here while it is checkmate, then we know that the side playing
    // has been checkmated.
    return if board.side_to_move() == Color::White {
        f32::MIN
    } else {
        f32::MAX
    };
}
pub(crate) fn new_eval_is_better(maximise: bool, old: &Evaluation, new: &Evaluation) -> bool {
    new.eval.is_some()
        && (old.eval.is_none()
            || maximise && new.eval.unwrap() > old.eval.unwrap()
            || !maximise && new.eval.unwrap() < old.eval.unwrap())
}
