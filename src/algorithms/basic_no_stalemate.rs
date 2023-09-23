use chess::{Action, Board, BoardStatus, Color, MoveGen};

use crate::common::algorithm::Algorithm;
use crate::common::utils::{self, vector_push_debug};

pub(crate) struct BasicNoStalemateAlgo;

impl BasicNoStalemateAlgo {
    fn node_eval_recursive(
        &self,
        board: &Board,
        depth: u32,
        analyze: bool,
    ) -> (Option<Action>, f32, Option<Vec<String>>) {
        if depth == 0 {
            return (None, self.eval(board), None);
        }

        // Whether we should try to maximise the eval
        let maximise: bool = board.side_to_move() == Color::White;

        let mut best_eval = (None, if maximise { f32::MIN } else { f32::MAX }, None);

        let board_status = board.status();
        if board_status == BoardStatus::Stalemate {
            best_eval = (None, 0., None)
        }
        // If we arrive at this position, and it is checkmate, then we know that the side playing
        // has been checkmated, and therefore the `best_eval` is correct. Because if we tried to
        // maximise, we failed, and if trying to minimise, we failed and therefore get the
        // lowest/highest eval

        for chess_move in MoveGen::new_legal(board) {
            let new_position = board.make_move_new(chess_move);
            let eval = self.node_eval_recursive(&new_position, depth - 1, false);

            if maximise && eval.1 > best_eval.1 || !maximise && eval.1 < best_eval.1 {
                if analyze {
                    let mut vec = Vec::new();
                    let new_best_move = chess_move.to_string();
                    if let Some(Action::MakeMove(previous_best_move)) = best_eval.0 {
                        let previous_best_move = previous_best_move.to_string();
                        utils::vector_push_debug!(
                            vec,
                            maximise,
                            previous_best_move,
                            best_eval.1,
                            new_best_move,
                            eval.1
                        );
                    } else {
                        utils::vector_push_debug!(
                            vec,
                            maximise,
                            best_eval.1,
                            new_best_move,
                            eval.1
                        );
                    }
                    best_eval.2 = Some(vec);
                }

                best_eval.1 = eval.1;
                best_eval.0 = Some(Action::MakeMove(chess_move));
            }
        }

        best_eval
    }

    fn next_action(
        &self,
        board: &Board,
        depth: u32,
        analyze: bool,
    ) -> (chess::Action, Vec<String>) {
        let out = self.node_eval_recursive(board, depth, analyze);
        let action = out.0.unwrap_or(Action::Resign(board.side_to_move()));
        let analyzer_data = out.2.unwrap_or_default();
        (action, analyzer_data)
    }
}

impl Algorithm for BasicNoStalemateAlgo {
    fn next_action(&self, board: &Board, analyze: bool) -> (chess::Action, Vec<String>) {
        self.next_action(board, 2, analyze)
    }

    fn eval(&self, board: &Board) -> f32 {
        let board_status = board.status();
        if board_status == BoardStatus::Stalemate {
            return 0.;
        }
        if board_status == BoardStatus::Checkmate {
            return if board.side_to_move() == Color::White {
                f32::MIN
            } else {
                f32::MAX
            };
        }
        let material_each_side = utils::material_each_side(board);

        // Negative when black has advantage
        let diff = material_each_side.0 as i32 - material_each_side.1 as i32;
        diff as f32
    }
}
