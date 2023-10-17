use std::time::Instant;

use chess::{Action, Board, BoardStatus, Color, MoveGen};

use crate::common::constants::modules::*;
use crate::common::utils::{self, Stats};

pub(crate) struct Algorithm {
    pub(crate) modules: u32,
}

impl Algorithm {
    pub(crate) fn new(modules: u32) -> Self {
        Self { modules }
    }

    fn node_eval_recursive(
        &self,
        board: &Board,
        depth: u32,
        mut alpha: f32,
        mut beta: f32,
        original: bool,
        deadline: Option<Instant>,
        stats: &mut Stats,
    ) -> (Option<Action>, f32, Option<Vec<String>>) {
        if depth == 0 {
            stats.leaves_visited += 1;
            return (None, self.eval(board), None);
        }

        // Whether we should try to maximise the eval
        let maximise: bool = board.side_to_move() == Color::White;
        let mut best_eval = (None, if maximise { f32::MIN } else { f32::MAX }, None);

        let legal_moves = MoveGen::new_legal(board);
        let num_legal_moves = legal_moves.len();
        if num_legal_moves == 0 && board.checkers().popcnt() == 0 {
            // Is Stalemate, no checking pieces
            best_eval = (None, 0., None)
        }
        // If we arrive at here and it is checkmate, then we know that the side playing
        // has been checkmated, and therefore the current `best_eval` is correct. Because if we tried to
        // maximise, we failed, and if trying to minimise, we failed and therefore get the
        // lowest/highest eval

        for (i, chess_move) in legal_moves.enumerate() {
            if deadline.map_or(false, |deadline| {
                !Instant::now().saturating_duration_since(deadline).is_zero()
            }) {
                // The previous value of progress_on_next_layer comes from deeper layers returning.
                // We want these contributions to be proportional to the contribution from a single
                // node on our layer
                stats.progress_on_next_layer *= 1. / num_legal_moves as f32;
                stats.progress_on_next_layer += i as f32 / num_legal_moves as f32;
                return best_eval;
            };

            let new_position = board.make_move_new(chess_move);
            let eval = self.node_eval_recursive(
                &new_position,
                depth - 1,
                alpha,
                beta,
                false,
                deadline,
                stats,
            );
            stats.nodes_visited += 1;

            if maximise && eval.1 > best_eval.1 || !maximise && eval.1 < best_eval.1 {
                if original && self.modules & ANALYZE != 0 {
                    let mut vec = Vec::new();
                    let new_best_move = chess_move.to_string();
                    let new_best_eval = eval.1;
                    utils::vector_push_debug!(
                        vec,
                        self.modules,
                        maximise,
                        best_eval.1,
                        new_best_move,
                        new_best_eval,
                    );
                    if let Some(Action::MakeMove(previous_best_move)) = best_eval.0 {
                        let previous_best_move = previous_best_move.to_string();
                        utils::vector_push_debug!(vec, previous_best_move);
                    }
                    best_eval.2 = Some(vec);
                }

                best_eval.1 = eval.1;
                best_eval.0 = Some(Action::MakeMove(chess_move));
            }

            if self.modules & ALPHA_BETA != 0 {
                if maximise {
                    // if eval.1 > alpha {
                    // println!("Alpha changed. {} -> {}. Beta: {}", alpha, eval.1, beta);
                    // }
                    alpha = alpha.max(eval.1);
                } else {
                    // if eval.1 < beta {
                    // println!("Beta changed. {} -> {}. Alpha: {}", beta, eval.1, alpha);
                    // }

                    beta = beta.min(eval.1);
                }

                if alpha > beta {
                    stats.alpha_beta_breaks += 1;
                    break;
                }
            }
        }

        best_eval
    }

    fn next_action_internal(
        &self,
        board: &Board,
        depth: u32,
        deadline: Option<Instant>,
    ) -> (chess::Action, Vec<String>, Stats) {
        let mut stats = Stats::default();
        let out =
            self.node_eval_recursive(board, depth, f32::MIN, f32::MAX, true, deadline, &mut stats);
        let action = out.0.unwrap_or(Action::Resign(board.side_to_move()));
        let analyzer_data = out.2.unwrap_or_default();
        (action, analyzer_data, stats)
    }

    pub(crate) fn next_action(
        &self,
        board: &Board,
        deadline: Instant,
    ) -> (chess::Action, Vec<String>, Stats) {
        const START_DEPTH: u32 = 1;
        let mut deepest_complete_output = self.next_action_internal(board, START_DEPTH, None);
        let mut deepest_complete_depth = START_DEPTH;
        for depth in (deepest_complete_depth + 1)..=10 {
            let latest_output = self.next_action_internal(board, depth, Some(deadline));
            let time_since_deadline = Instant::now().saturating_duration_since(deadline);
            if !time_since_deadline.is_zero() {
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
