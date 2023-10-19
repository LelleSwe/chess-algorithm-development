use std::collections::HashMap;
use std::time::Instant;

use chess::{Action, Board, BoardStatus, ChessMove, Color, MoveGen};

use crate::common::constants::modules::*;
use crate::common::utils::{self, Stats};

use super::utils::{Evaluation, TranspositionEntry};

pub(crate) struct Algorithm {
    pub(crate) modules: u32,
    transposition_table: HashMap<Board, TranspositionEntry>,
}

impl Algorithm {
    pub(crate) fn new(modules: u32) -> Self {
        Self {
            modules,
            transposition_table: HashMap::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn node_eval_recursive(
        &mut self,
        board: &Board,
        depth: u32,
        mut alpha: f32,
        mut beta: f32,
        original: bool,
        deadline: Option<Instant>,
        stats: &mut Stats,
    ) -> Evaluation {
        if depth == 0 {
            stats.leaves_visited += 1;
            return Evaluation::new(self.eval(board), None, None);
        }

        // Whether we should try to maximise the eval
        let maximise: bool = board.side_to_move() == Color::White;
        let mut best_evaluation =
            Evaluation::new(if maximise { f32::MIN } else { f32::MAX }, None, None);

        let legal_moves = MoveGen::new_legal(board);
        let num_legal_moves = legal_moves.len();
        if num_legal_moves == 0 {
            if board.checkers().popcnt() == 0 {
                // Is Stalemate, no checking pieces
                best_evaluation.eval = 0.;
            }
            // If we arrive at here and it is checkmate, then we know that the side playing
            // has been checkmated, and therefore the current `best_eval` is correct. Because if we tried to
            // maximise, we failed, and if trying to minimise, we failed and therefore get the
            // lowest/highest eval
            println!("The thing happened");
            return best_evaluation;
        }

        let mut boards = legal_moves
            .map(|chess_move| {
                let board = board.make_move_new(chess_move);
                let mut transposition_entry = None;
                if self.modules & TRANSPOSITION_TABLE != 0 {
                    transposition_entry = self.transposition_table.get(&board);
                }
                (chess_move, board, transposition_entry)
            })
            .collect::<Vec<(ChessMove, Board, Option<&TranspositionEntry>)>>();

        // Sort by eval
        boards.sort_by(|board1, board2| {
            let ordering = board1
                .2
                .map_or(0., |entry| entry.eval)
                .partial_cmp(&board2.2.map_or(0., |entry| entry.eval))
                .expect("Eval is a valid value");

            if !maximise {
                return ordering.reverse();
            }
            ordering
        });

        for (i, (chess_move, new_board, transposition_entry)) in boards.iter().enumerate() {
            if deadline.map_or(false, |deadline| {
                !Instant::now().saturating_duration_since(deadline).is_zero()
            }) {
                // The previous value of progress_on_next_layer comes from deeper layers returning.
                // We want these contributions to be proportional to the contribution from a single
                // node on our layer
                stats.progress_on_next_layer *= 1. / num_legal_moves as f32;
                stats.progress_on_next_layer += i as f32 / num_legal_moves as f32;
                return best_evaluation;
            };

            let evaluation = if transposition_entry.is_none()
                || transposition_entry.unwrap().depth < depth
            {
                self.node_eval_recursive(new_board, depth - 1, alpha, beta, false, deadline, stats)
            } else {
                Evaluation::new(
                    transposition_entry.unwrap().eval,
                    transposition_entry.unwrap().next_action,
                    None,
                )
            };
            stats.nodes_visited += 1;

            // Replace best_eval if ours is better
            if maximise && evaluation.eval > best_evaluation.eval
                || !maximise && evaluation.eval < best_evaluation.eval
            {
                if original && self.modules & ANALYZE != 0 {
                    let mut vec = Vec::new();
                    let new_best_move = chess_move.to_string();
                    let new_best_eval = evaluation.eval;
                    utils::vector_push_debug!(
                        vec,
                        self.modules,
                        maximise,
                        best_evaluation.eval,
                        new_best_move,
                        new_best_eval,
                    );
                    if let Some(Action::MakeMove(previous_best_move)) = best_evaluation.next_action
                    {
                        let previous_best_move = previous_best_move.to_string();
                        utils::vector_push_debug!(vec, previous_best_move);
                    }
                    best_evaluation.debug_data = Some(vec);
                }

                best_evaluation.eval = evaluation.eval;
                best_evaluation.next_action = Some(Action::MakeMove(*chess_move));
            }

            if self.modules & ALPHA_BETA != 0 {
                if maximise {
                    alpha = alpha.max(evaluation.eval);
                } else {
                    beta = beta.min(evaluation.eval);
                }

                if alpha > beta {
                    stats.alpha_beta_breaks += 1;
                    break;
                }
            }
        }

        self.transposition_table.insert(
            *board,
            TranspositionEntry::new(depth, best_evaluation.eval, best_evaluation.next_action),
        );
        best_evaluation
    }

    fn next_action_internal(
        &mut self,
        board: &Board,
        depth: u32,
        deadline: Option<Instant>,
    ) -> (chess::Action, Vec<String>, Stats) {
        let mut stats = Stats::default();
        let out =
            self.node_eval_recursive(board, depth, f32::MIN, f32::MAX, true, deadline, &mut stats);
        let action = if out.next_action.is_none() {
            match board.status() {
                BoardStatus::Ongoing => {
                    panic!("No action returned by algorithm even though game is still ongoing")
                }
                BoardStatus::Stalemate => Action::DeclareDraw,
                BoardStatus::Checkmate => Action::Resign(board.side_to_move()),
            }
        } else {
            out.next_action.unwrap()
        };
        let analyzer_data = out.debug_data.unwrap_or_default();
        (action, analyzer_data, stats)
    }

    pub(crate) fn next_action(
        &mut self,
        board: &Board,
        deadline: Instant,
    ) -> (chess::Action, Vec<String>, Stats) {
        // Guarantee that at least the first layer gets done.
        const START_DEPTH: u32 = 1;
        let mut deepest_complete_output = self.next_action_internal(board, START_DEPTH, None);
        let mut deepest_complete_depth = START_DEPTH;

        for depth in (deepest_complete_depth + 1)..=10 {
            let latest_output = self.next_action_internal(board, depth, Some(deadline));
            if utils::passed_deadline(deadline) {
                // The cancelled layer is the one with this data
                deepest_complete_output.2.progress_on_next_layer =
                    latest_output.2.progress_on_next_layer;
                break;
            } else {
                deepest_complete_output = latest_output;
                deepest_complete_depth = depth;
            }
        }
        deepest_complete_output.2.depth = deepest_complete_depth;
        deepest_complete_output
    }

    pub(crate) fn eval(&self, board: &Board) -> f32 {
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
