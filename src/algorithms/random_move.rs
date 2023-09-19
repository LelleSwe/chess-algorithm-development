use chess::{Action, ChessMove, Game};
use rand::Rng;

use crate::common::algorithm::Algorithm;

#[derive(Debug, Default)]
pub(crate) struct RandomMove;

impl Algorithm for RandomMove {
    fn next_move(&self, game: &Game) -> Action {
        let legal_moves: Vec<ChessMove> =
            chess::MoveGen::new_legal(&game.current_position()).collect();
        let index = rand::thread_rng().gen_range(0..legal_moves.len());

        Action::MakeMove(legal_moves[index])
    }

    fn eval(&self, _game: &Game) -> f32 {
        unimplemented!()
    }
}

#[derive(Debug, Default)]
pub(crate) struct RandomMoveClaimDraw;

impl Algorithm for RandomMoveClaimDraw {
    fn next_move(&self, game: &Game) -> Action {
        if game.can_declare_draw() {
            return Action::DeclareDraw;
        }
        let legal_moves: Vec<ChessMove> =
            chess::MoveGen::new_legal(&game.current_position()).collect();
        let index = rand::thread_rng().gen_range(0..legal_moves.len());

        Action::MakeMove(legal_moves[index])
    }

    fn eval(&self, _game: &Game) -> f32 {
        unimplemented!()
    }
}

pub(crate) struct InstaResign;

impl Algorithm for InstaResign {
    fn next_move(&self, game: &Game) -> Action {
        Action::Resign(game.side_to_move())
    }

    fn eval(&self, _game: &Game) -> f32 {
        unimplemented!()
    }
}
