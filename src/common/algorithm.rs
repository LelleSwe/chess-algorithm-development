use chess::{Action, Game};

pub(crate) trait Algorithm {
    fn next_move(&self, game: &Game) -> Action;
    fn eval(&self, game: &Game) -> f32;
}
