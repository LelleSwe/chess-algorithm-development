use std::{collections::HashMap, cmp};
use tokio::time::{Duration, Instant};
use rustc_hash::FxHasher;

use chess::{Action, Board, BoardStatus, ChessMove, Color, MoveGen, Piece, BitBoard};

use crate::common::constants::{modules::{self, *}, naive_psqt_tables::*, tapered_pesto_psqt_tables::*};
use crate::common::utils::{self, module_enabled, Stats};

use super::utils::{Evaluation, TranspositionEntry};

#[derive(Clone, Debug)]
pub(crate) struct Algorithm {
    pub(crate) modules: u32,
    transposition_table: HashMap<Board, TranspositionEntry>,
    pub(crate) time_per_move: Duration,
    /// Number of times that a given board has been played
    pub(crate) board_played_times: HashMap<Board, u32>,
    pub(crate) pawn_hash: HashMap<BitBoard, f32>,
    pub(crate) naive_psqt_pawn_hash: HashMap<BitBoard, f32>,
    pub(crate) naive_psqt_rook_hash: HashMap<BitBoard, f32>,
    pub(crate) naive_psqt_king_hash: HashMap<BitBoard, f32>,
    pub(crate) naive_psqt_queen_hash: HashMap<BitBoard, f32>,
    pub(crate) naive_psqt_knight_hash: HashMap<BitBoard, f32>,
    pub(crate) naive_psqt_bishop_hash: HashMap<BitBoard, f32>

}

impl Algorithm {
    pub(crate) fn new(modules: u32, time_per_move: Duration) -> Self {
        Self {
            modules,
            transposition_table: HashMap::with_capacity(45),
            time_per_move,
            board_played_times: HashMap::new(),
            pawn_hash: HashMap::new(),
            naive_psqt_knight_hash: HashMap::new(),
            naive_psqt_pawn_hash: HashMap::new(),
            naive_psqt_rook_hash: HashMap::new(),
            naive_psqt_bishop_hash: HashMap::new(),
            naive_psqt_queen_hash: HashMap::new(),
            naive_psqt_king_hash: HashMap::new(),
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
            let eval = self.eval(board, board_played_times_prediction, 0.);
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
            
            //best_evaluation.eval = Some(f32::MIN);


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

            if depth > stats.max_depth {
                stats.max_depth = depth;
            }

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

            if utils::module_enabled(self.modules, modules::TAPERED_INCREMENTAL_PESTO_PSQT) {

            }
        }

        (action, deepest_complete_output.1, deepest_complete_output.2)
    }

    pub(crate) fn eval(
        &mut self,
        board: &Board,
        board_played_times_prediction: &HashMap<Board, u32>,
        incremental_eval: f32,
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
        let mut naive_psqt: f32 = 0.;
        if utils::module_enabled(self.modules, modules::NAIVE_PSQT) {
            fn naive_psqt_calc(naive_psqt_table: [f32; 64], piece_bitboard: &BitBoard, color_bitboard: &BitBoard) -> f32 {
                //Essentially, gets the dot product between a "vector" of the bitboard (containing 64 0s and 1s) and the table with NAIVE_PSQT bonus constants.
                let mut bonus: f32 = 0.;
                //Get's the bitboard with all piece NAIVE_PSQTs, and runs bitwise and for the board having one's own colors.
                for i in 0..63 {
                    //The naive_psqt table and bitboard are flipped vertically, hence .reverse_colors(). Reverse colors is for some reason faster than replacing i with 56-i+2*(i%8).
                    bonus += ((piece_bitboard & color_bitboard).reverse_colors().to_size(0) >> i & 1) as f32 * naive_psqt_table[i]; 
                }
                return bonus;
            }

            //Utilizes hashmaps so we don't have to recalculate the entire bonus for all pieces every move. This is slightly faster.
            fn in_hash_map(bitboard: &BitBoard, color_bitboard: &BitBoard, naive_psqt_table: [f32; 64], naive_psqt_hash_map: &mut HashMap::<BitBoard, f32>) -> f32 {
                if !naive_psqt_hash_map.contains_key(&(bitboard & color_bitboard)) {
                    naive_psqt_hash_map.insert(bitboard & color_bitboard, naive_psqt_calc(naive_psqt_table, bitboard, color_bitboard));
                }
                return *naive_psqt_hash_map.get(&(bitboard & color_bitboard)).unwrap();
            }

            if board.side_to_move() == Color::White {
                naive_psqt += in_hash_map(board.pieces(Piece::Pawn), board.color_combined(Color::White), NAIVE_PSQT_TABLE_PAWN, &mut self.naive_psqt_pawn_hash);
                naive_psqt += in_hash_map(board.pieces(Piece::Rook), board.color_combined(Color::White), NAIVE_PSQT_TABLE_ROOK, &mut self.naive_psqt_rook_hash);
                naive_psqt += in_hash_map(board.pieces(Piece::King), board.color_combined(Color::White), NAIVE_PSQT_TABLE_KING, &mut self.naive_psqt_king_hash);
                naive_psqt += in_hash_map(board.pieces(Piece::Queen), board.color_combined(Color::White), NAIVE_PSQT_TABLE_QUEEN, &mut self.naive_psqt_queen_hash);
                naive_psqt += in_hash_map(board.pieces(Piece::Bishop), board.color_combined(Color::White), NAIVE_PSQT_TABLE_BISHOP, &mut self.naive_psqt_bishop_hash);
                naive_psqt += in_hash_map(board.pieces(Piece::Knight), board.color_combined(Color::White), NAIVE_PSQT_TABLE_KNIGHT, &mut self.naive_psqt_knight_hash);
            } else {
                naive_psqt += in_hash_map(board.pieces(Piece::Pawn), board.color_combined(Color::Black), NAIVE_PSQT_TABLE_PAWN, &mut self.naive_psqt_pawn_hash);
                naive_psqt += in_hash_map(board.pieces(Piece::Rook), board.color_combined(Color::Black), NAIVE_PSQT_TABLE_ROOK, &mut self.naive_psqt_rook_hash);
                naive_psqt += in_hash_map(board.pieces(Piece::King), board.color_combined(Color::Black), NAIVE_PSQT_TABLE_KING, &mut self.naive_psqt_king_hash);
                naive_psqt += in_hash_map(board.pieces(Piece::Queen), board.color_combined(Color::Black), NAIVE_PSQT_TABLE_QUEEN, &mut self.naive_psqt_queen_hash);
                naive_psqt += in_hash_map(board.pieces(Piece::Bishop), board.color_combined(Color::Black), NAIVE_PSQT_TABLE_BISHOP, &mut self.naive_psqt_bishop_hash);
                naive_psqt += in_hash_map(board.pieces(Piece::Knight), board.color_combined(Color::Black), NAIVE_PSQT_TABLE_KNIGHT, &mut self.naive_psqt_knight_hash);
            }
        }

        let mut tapered_pesto: f32 = 0.;
        if utils::module_enabled(self.modules, modules::TAPERED_EVERY_PRESTO_PSQT) {
            fn tapered_psqt_calc(piece_bitboard: &BitBoard, color_bitboard: &BitBoard, material: (u32, u32), tapered_table_mg: [f32; 64], tapered_table_eg: [f32; 64]) -> f32 {
                //Essentially, gets the dot product between a "vector" of the bitboard (containing 64 0s and 1s) and the table with NAIVE_PSQT bonus constants.
                let mut bonus: f32 = 0.;
                //Get's the bitboard with all piece NAIVE_PSQTs, and runs bitwise and for the board having one's own colors.
                for i in 0..63 {
                    //The psqt tables and bitboards are flipped vertically, hence .reverse_colors(). Reverse colors is for some reason faster than replacing i with 56-i+2*(i%8).
                    //By being tapered, it means that we have an opening + middlgame and an endgame PSQT, and we (hopefully?) linerarly transition from one to the other, depending on material value.
                    bonus += ((piece_bitboard & color_bitboard).reverse_colors().to_size(0) >> i & 1) as f32 * 
                             ((material.0 + material.1 - 2000) as f32 / 78. * tapered_table_mg[i] + (material.0 + material.1 - 2000 + 78) as f32 / 78. * tapered_table_eg[i]) as f32; 
                }
                return bonus;
            }

            if board.side_to_move() == Color::White {
                tapered_pesto += tapered_psqt_calc(board.pieces(Piece::Pawn), board.color_combined(Color::White), material_each_side, TAPERED_MG_PESTO_PAWN, TAPERED_EG_PESTO_PAWN);
                tapered_pesto += tapered_psqt_calc(board.pieces(Piece::Rook), board.color_combined(Color::White), material_each_side, TAPERED_MG_PESTO_ROOK, TAPERED_EG_PESTO_ROOK);
                tapered_pesto += tapered_psqt_calc(board.pieces(Piece::King), board.color_combined(Color::White), material_each_side, TAPERED_MG_PESTO_KING, TAPERED_EG_PESTO_KING);
                tapered_pesto += tapered_psqt_calc(board.pieces(Piece::Queen), board.color_combined(Color::White), material_each_side, TAPERED_MG_PESTO_QUEEN, TAPERED_EG_PESTO_QUEEN);
                tapered_pesto += tapered_psqt_calc(board.pieces(Piece::Bishop), board.color_combined(Color::White), material_each_side, TAPERED_MG_PESTO_BISHOP, TAPERED_EG_PESTO_BISHOP);
                tapered_pesto += tapered_psqt_calc(board.pieces(Piece::Knight), board.color_combined(Color::White), material_each_side, TAPERED_MG_PESTO_KNIGHT, TAPERED_EG_PESTO_KNIGT);
            } else {
                tapered_pesto += tapered_psqt_calc(board.pieces(Piece::Pawn), board.color_combined(Color::Black), material_each_side, TAPERED_MG_PESTO_PAWN, TAPERED_EG_PESTO_PAWN);
                tapered_pesto += tapered_psqt_calc(board.pieces(Piece::Rook), board.color_combined(Color::Black), material_each_side, TAPERED_MG_PESTO_ROOK, TAPERED_EG_PESTO_ROOK);
                tapered_pesto += tapered_psqt_calc(board.pieces(Piece::King), board.color_combined(Color::Black), material_each_side, TAPERED_MG_PESTO_KING, TAPERED_EG_PESTO_KING);
                tapered_pesto += tapered_psqt_calc(board.pieces(Piece::Queen), board.color_combined(Color::Black), material_each_side, TAPERED_MG_PESTO_QUEEN, TAPERED_EG_PESTO_QUEEN);
                tapered_pesto += tapered_psqt_calc(board.pieces(Piece::Bishop), board.color_combined(Color::Black), material_each_side, TAPERED_MG_PESTO_BISHOP, TAPERED_EG_PESTO_BISHOP);
                tapered_pesto += tapered_psqt_calc(board.pieces(Piece::Knight), board.color_combined(Color::Black), material_each_side, TAPERED_MG_PESTO_KNIGHT, TAPERED_EG_PESTO_KNIGT);
            }
        }

        let mut pawn_structure: f32 = 0.;
        if utils::module_enabled(self.modules, modules::PAWN_STRUCTURE) {
            fn pawn_structure_calc(all_pawn_bitboard: &BitBoard, color_bitboard: &BitBoard, all_king_bitboard: &BitBoard) -> f32 {
                let mut bonus: f32 = 0.;
                let pawn_bitboard: usize = (all_pawn_bitboard & color_bitboard).to_size(0);
                let king_bitboard: usize = (all_king_bitboard & color_bitboard).to_size(0);
                //pawn chain, awarding 0.5 eval for each pawn protected by another pawn.
                bonus += 0.5*((pawn_bitboard & (pawn_bitboard << 7)).count_ones() + (pawn_bitboard & (pawn_bitboard << 9)).count_ones()) as f32;

                //stacked pawns. -0.5 points per rank containing >1 pawns. By taking the pawn bitboard and operating bitwise AND for another bitboard (integer) where the leftmost rank is filled. This returns all pawns in that rank. By bitshifting we can choose rank. Additionally by counting we get number of pawns. We then remove 1 as we only want to know if there are >1 pawn. If there is, subtract 0.5 points per extra pawn.
                for i in 0..7 {
                    //constant 9259542123273814144 = 0x8080808080808080, or the entire first rank.
                    bonus -= 0.5*cmp::max((pawn_bitboard & (0x8080808080808080 >> i)).count_ones() as i64 - 1, 0) as f32;
                }

                //king safety. Outer 3 pawns get +1 eval bonus per pawn if king is behind them. King naive_psqt required is either ..X..... or ......X.
                bonus += ((king_bitboard & 0x2).count_ones() * (pawn_bitboard & 0x107).count_ones() + (king_bitboard & 0x20).count_ones() * (pawn_bitboard & 0x80E000).count_ones()) as f32;
                return bonus;
            }

            //Because pawn moves (according to chessprogramming.org) are rarely performed, hashing them is useful.
            if board.side_to_move() == Color::White {
                let pawn_bitboard: BitBoard = board.pieces(Piece::Pawn) & board.color_combined(Color::White);
                if !self.pawn_hash.contains_key(&pawn_bitboard) {
                    self.pawn_hash.insert(pawn_bitboard, pawn_structure_calc(board.pieces(Piece::Pawn), board.color_combined(Color::White), board.pieces(Piece::King)));
                }
                pawn_structure = *self.pawn_hash.get(&pawn_bitboard).unwrap();
            } else {
                let pawn_bitboard: BitBoard = board.pieces(Piece::Pawn) & board.color_combined(Color::Black);
                if !self.pawn_hash.contains_key(&pawn_bitboard) {
                    self.pawn_hash.insert(pawn_bitboard, pawn_structure_calc(board.pieces(Piece::Pawn), board.color_combined(Color::Black), board.pieces(Piece::King)));
                }
                pawn_structure = *self.pawn_hash.get(&pawn_bitboard).unwrap();
            }
        }

        let evaluation: f32 = controlled_squares as f32 / 20. + diff_material as f32 + naive_psqt + pawn_structure + tapered_pesto;
        return evaluation
    }

    pub(crate) fn reset(&mut self) {
        self.transposition_table = HashMap::new();
        self.board_played_times = HashMap::new();
        self.pawn_hash = HashMap::new();
        self.naive_psqt_pawn_hash = HashMap::new();
        self.naive_psqt_king_hash = HashMap::new();
        self.naive_psqt_queen_hash = HashMap::new();
        self.naive_psqt_bishop_hash = HashMap::new();
        self.naive_psqt_rook_hash = HashMap::new();
        self.naive_psqt_knight_hash = HashMap::new();
    }
}
