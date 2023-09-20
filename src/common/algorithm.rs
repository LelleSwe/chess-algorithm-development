use chess::{Action, Board};

pub(crate) trait Algorithm {
    fn next_move(&self, board: &Board) -> Action;
    fn eval(&self, board: &Board) -> f32;
}
