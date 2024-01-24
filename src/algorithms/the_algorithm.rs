use std::collections::HashMap;
use tokio::time::{Duration, Instant};

use chess::{Action, Board, BoardStatus, ChessMove, Color, MoveGen, Piece, BitBoard};

use crate::common::constants::{modules::{self, *}, position_bonus_tables::*};
use crate::common::utils::{self, module_enabled, Stats};

use super::utils::{Evaluation, TranspositionEntry};

#[derive(Clone, Debug)]
pub(crate) struct Algorithm {
    pub(crate) modules: u32,
    transposition_table: HashMap<Board, TranspositionEntry>,
    pub(crate) time_per_move: Duration,
    /// Number of times that a given board has been played
    pub(crate) board_played_times: HashMap<Board, u32>,
}

impl Algorithm {
    pub(crate) fn new(modules: u32, time_per_move: Duration) -> Self {
        Self {
            modules,
            transposition_table: HashMap::with_capacity(45),
            time_per_move,
            board_played_times: HashMap::new(),
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
        num_extensions: u32,
        board_played_times_prediction: &mut HashMap<Board, u32>,
    ) -> Evaluation {
        if depth == 0 {
            stats.leaves_visited += 1;
            let eval = self.eval(board, board_played_times_prediction);
            // if module_enabled(self.modules, TRANSPOSITION_TABLE) {
            //     let start = Instant::now();
            //     self.transposition_table
            //         .insert(*board, TranspositionEntry::new(depth, eval, None));
            //     stats.time_for_transposition_access += Instant::now() - start;
            //     stats.transposition_table_entries += 1
            // }
            return Evaluation::new(Some(eval), None, None);
        }

        // Whether we should try to maximise the eval
        let maximise: bool = board.side_to_move() == Color::White;
        let mut best_evaluation = Evaluation::new(None, None, None);

        let legal_moves = MoveGen::new_legal(board);
        let num_legal_moves = legal_moves.len();
        if num_legal_moves == 0 {
            if board.checkers().popcnt() == 0 {
                // Is Stalemate, no checking pieces
                best_evaluation.eval = Some(0.);
            }

            // If we arrive at here and it is checkmate, then we know that the side playing
            // has been checkmated.

            best_evaluation.eval = Some(if board.side_to_move() == Color::White {
                f32::MIN
            } else {
                f32::MAX
            });
            return best_evaluation;
        }

        let mut boards = legal_moves
            .map(|chess_move| {
                let board = board.make_move_new(chess_move);
                let mut transposition_entry = None;
                if module_enabled(self.modules, TRANSPOSITION_TABLE) {
                    let start = Instant::now();

                    transposition_entry = self.transposition_table.get(&board).copied();

                    let time_for_transposition_access = Instant::now() - start;
                    stats.time_for_transposition_access += time_for_transposition_access;
                }
                (chess_move, board, transposition_entry)
            })
            .collect::<Vec<(ChessMove, Board, Option<TranspositionEntry>)>>();

        // Sort by eval
        boards.sort_by(|board1, board2| {
            let eval1 = board1.2.map_or(0., |entry| entry.eval);
            let eval2 = board2.2.map_or(0., |entry| entry.eval);
            let ordering = eval1.partial_cmp(&eval2).expect("Eval is a valid value");

            if maximise {
                return ordering.reverse();
            }
            ordering
        });

        for (i, (chess_move, new_board, transposition_entry)) in boards.into_iter().enumerate() {
            if deadline.is_some_and(utils::passed_deadline) {
                // The previous value of progress_on_next_layer comes from deeper layers returning.
                // We want these contributions to be proportional to the contribution from a single
                // node on our layer
                stats.progress_on_next_layer *= 1. / num_legal_moves as f32;
                stats.progress_on_next_layer +=
                    (i.saturating_sub(1)) as f32 / num_legal_moves as f32;
                return best_evaluation;
            };
            if module_enabled(self.modules, SKIP_BAD_MOVES)
                && i as f32 > num_legal_moves as f32 * 1.
            {
                return best_evaluation;
            }

            let extend_by =
                if !module_enabled(self.modules, SEARCH_EXTENSIONS) || num_extensions > 3 {
                    0
                } else if num_legal_moves == 1 || new_board.checkers().popcnt() >= 2 {
                    1
                } else {
                    0
                };

            let evaluation =
                if transposition_entry.is_some() && transposition_entry.unwrap().depth >= depth {
                    stats.transposition_table_accesses += 1;
                    Evaluation::new(
                        Some(transposition_entry.unwrap().eval),
                        transposition_entry.unwrap().next_action,
                        None,
                    )
                } else {
                    board_played_times_prediction.insert(
                        new_board,
                        *board_played_times_prediction.get(&new_board).unwrap_or(&0) + 1,
                    );
                    let evaluation = self.node_eval_recursive(
                        &new_board,
                        depth - 1 + extend_by,
                        alpha,
                        beta,
                        false,
                        deadline,
                        stats,
                        num_extensions + extend_by,
                        board_played_times_prediction,
                    );
                    board_played_times_prediction.insert(
                        new_board,
                        *board_played_times_prediction.get(&new_board).unwrap_or(&0) - 1,
                    );
                    evaluation
                };
            stats.nodes_visited += 1;

            // Replace best_eval if ours is better
            if evaluation.eval.is_some()
                && (best_evaluation.eval.is_none()
                    || maximise && evaluation.eval.unwrap() > best_evaluation.eval.unwrap()
                    || !maximise && evaluation.eval.unwrap() < best_evaluation.eval.unwrap())
            {
                if original && module_enabled(self.modules, ANALYZE) {
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
                best_evaluation.next_action = Some(Action::MakeMove(chess_move));
            }

            if module_enabled(self.modules, ALPHA_BETA) {
                if let Some(eval) = evaluation.eval {
                    if maximise {
                        alpha = alpha.max(eval);
                    } else {
                        beta = beta.min(eval);
                    }
                }

                if alpha > beta {
                    stats.alpha_beta_breaks += 1;
                    break;
                }
            }
        }

        if module_enabled(self.modules, TRANSPOSITION_TABLE) && depth >= 3 {
            if let Some(best_eval) = best_evaluation.eval {
                let start = Instant::now();
                self.transposition_table.insert(
                    *board,
                    TranspositionEntry::new(depth, best_eval, best_evaluation.next_action),
                );
                stats.time_for_transposition_access += Instant::now() - start;
            }
            stats.transposition_table_entries += 1
        }

        if best_evaluation.debug_data.is_some() {
            let mut debug_data = best_evaluation.debug_data.take().unwrap();
            if let Some(Action::MakeMove(next_move)) = best_evaluation.next_action {
                utils::vector_push_debug!(debug_data, best_evaluation.eval, next_move.to_string(),);
                best_evaluation.debug_data = Some(debug_data);
            }
        }
        best_evaluation
    }

    fn next_action(
        &mut self,
        board: &Board,
        depth: u32,
        deadline: Option<Instant>,
    ) -> (Option<chess::Action>, Vec<String>, Stats) {
        let mut stats = Stats::default();
        let out = self.node_eval_recursive(
            board,
            depth,
            f32::MIN,
            f32::MAX,
            true,
            deadline,
            &mut stats,
            0,
            &mut HashMap::new(),
        );
        let analyzer_data = out.debug_data.unwrap_or_default();
        (out.next_action, analyzer_data, stats)
    }

    pub(crate) fn next_action_iterative_deepening(
        &mut self,
        board: &Board,
        deadline: Instant,
    ) -> (chess::Action, Vec<String>, Stats) {
        self.board_played_times.insert(
            *board,
            *self.board_played_times.get(board).unwrap_or(&0) + 1,
        );

        // Guarantee that at least the first layer gets done.
        const START_DEPTH: u32 = 1;
        let mut deepest_complete_output = self.next_action(board, START_DEPTH, None);
        let mut deepest_complete_depth = START_DEPTH;

        for depth in (deepest_complete_depth + 1)..=10 {
            let latest_output = self.next_action(board, depth, Some(deadline));
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
        deepest_complete_output.2.tt_size = self.transposition_table.len() as u32;

        let mut action = match deepest_complete_output.0 {
            Some(action) => action,
            None => match board.status() {
                BoardStatus::Ongoing => {
                    println!("{}", board);
                    println!("{:#?}", deepest_complete_output.1);
                    panic!("No action returned by algorithm even though game is still ongoing")
                }
                BoardStatus::Stalemate => Action::DeclareDraw,
                BoardStatus::Checkmate => Action::Resign(board.side_to_move()),
            },
        };

        if let Action::MakeMove(chess_move) = action {
            let new_board = board.make_move_new(chess_move);
            let old_value = *self.board_played_times.get(&new_board).unwrap_or(&0);
            if old_value >= 3 {
                // Oh no! We should declare draw by three-fold repetition. This is not checked
                // unless we do this.
                action = Action::DeclareDraw;
            }
            self.board_played_times.insert(new_board, old_value + 1);
        }

        (action, deepest_complete_output.1, deepest_complete_output.2)
    }

    pub(crate) fn eval(
        &self,
        board: &Board,
        board_played_times_prediction: &HashMap<Board, u32>,
    ) -> f32 {
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
        let board_played_times = *self.board_played_times.get(board).unwrap_or(&0)
            + *board_played_times_prediction.get(board).unwrap_or(&0);
        if board_played_times >= 2 {
            // This is third time this is played. Draw by three-fold repetition
            return 0.;
        }
        let material_each_side = utils::material_each_side(board);

        // Negative when black has advantage
        let diff_material = material_each_side.0 as i32 - material_each_side.1 as i32;

        let mut controlled_squares = 0;
        if utils::module_enabled(self.modules, modules::SQUARE_CONTROL_METRIC) {
            controlled_squares = if board.side_to_move() == Color::Black {
                -1i32
            } else {
                1i32
            } * MoveGen::new_legal(board).count() as i32;
        }

        //Compares piece position with an 8x8 table containing certain values. The value corresponding to the position of the piece gets added as evaluation.
        let mut position_bonus: f32 = 0.;
        if utils::module_enabled(self.modules, modules::POSITION_BONUS) {
            fn position_bonus_calc(position_table: [f32; 64], bitboard: &BitBoard) -> f32 {
                //Essentially, gets the dot product between a "vector" of the bitboard (containing 64 0s and 1s) and the table with position bonus constants.
                let mut bonus: f32 = 0.;
                for i in 0..63 {
                    //I'm pretty sure the bitboard and position_table have opposite orientationns. Regardless, flipping the bitboard significantly increased performance. I don't know if the bitboard gets only 1 colour, or both. If both then for some reason this still improves things.
                    bonus += ((bitboard.reverse_colors().to_size(63) as u64) >> i & 1) as f32 * position_table[i]; 
                }
                return bonus;
            }
            position_bonus += position_bonus_calc(position_bonus_table_pawn, board.pieces(Piece::Pawn));
            position_bonus += position_bonus_calc(position_bonus_table_knight, board.pieces(Piece::Knight));
            position_bonus += position_bonus_calc(position_bonus_table_rook, board.pieces(Piece::Rook));
            position_bonus += position_bonus_calc(position_bonus_table_king, board.pieces(Piece::King));
            position_bonus += position_bonus_calc(position_bonus_table_queen, board.pieces(Piece::Queen));
            position_bonus += position_bonus_calc(position_bonus_table_bishop, board.pieces(Piece::Bishop));

        }

        let evaluation: f32 = controlled_squares as f32 / 20. + diff_material as f32 + position_bonus;
        return evaluation
    }

    pub(crate) fn reset(&mut self) {
        self.transposition_table = HashMap::new();
        self.board_played_times = HashMap::new();
    }
}
