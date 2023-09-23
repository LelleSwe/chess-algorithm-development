use std::time::Instant;

use chess::{Action, Board, Color, MoveGen};

use crate::common::algorithm::Algorithm;
use crate::common::utils;

pub(crate) struct BasicAlgo;

impl BasicAlgo {
    fn node_eval_recursive(&self, board: &Board, depth: u32) -> (Option<Action>, f32) {
        if depth == 0 {
            return (None, self.eval(board));
        }

        // Whether we should try to maximise the eval
        let maximise: bool = board.side_to_move() == Color::White;

        let mut best_eval = (None, if maximise { f32::MIN } else { f32::MAX });
        for chess_move in MoveGen::new_legal(board) {
            let new_position = board.make_move_new(chess_move);
            let eval = self.node_eval_recursive(&new_position, depth - 1);

            if maximise && eval.1 > best_eval.1 || !maximise && eval.1 <= best_eval.1 {
                best_eval = eval;
                best_eval.0 = Some(Action::MakeMove(chess_move));
            }
        }

        best_eval
    }

    fn next_action(&self, board: &Board, depth: u32, _analyze: bool) -> (Action, Vec<String>) {
        let out = self
            .node_eval_recursive(board, depth)
            .0
            .unwrap_or(Action::Resign(board.side_to_move()));
        (out, Vec::new())
    }
}

impl Algorithm for BasicAlgo {
    fn next_action(
        &self,
        board: &Board,
        analyze: bool,
        _deadline: Instant,
    ) -> (chess::Action, Vec<String>) {
        self.next_action(board, 2, analyze)
    }

    fn eval(&self, board: &Board) -> f32 {
        let material_each_side = utils::material_each_side(board);

        // Negative when black has advantage
        let diff = material_each_side.0 as i32 - material_each_side.1 as i32;
        diff as f32
    }
}
