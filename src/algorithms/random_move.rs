use chess::{Action, Board, ChessMove};
use rand::Rng;

use crate::common::algorithm::Algorithm;

#[derive(Debug, Default)]
pub(crate) struct RandomMove;

impl Algorithm for RandomMove {
    fn next_move(&self, board: &Board) -> Action {
        let legal_moves: Vec<ChessMove> = chess::MoveGen::new_legal(board).collect();
        let index = rand::thread_rng().gen_range(0..legal_moves.len());

        Action::MakeMove(legal_moves[index])
    }

    fn eval(&self, _game: &Board) -> f32 {
        unimplemented!()
    }
}

pub(crate) struct InstaResign;

impl Algorithm for InstaResign {
    fn next_move(&self, board: &Board) -> Action {
        Action::Resign(board.side_to_move())
    }

    fn eval(&self, _board: &Board) -> f32 {
        unimplemented!()
    }
}
