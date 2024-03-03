use std::collections::HashMap;

use chess::{Action, BitBoard, Board, BoardStatus, ChessMove, Color, MoveGen, Piece};
use tokio::time::{Duration, Instant};

use crate::algorithms::{draw_checker, eval};
use crate::common::constants::{modules::*, naive_psqt_tables::*, tapered_pesto_psqt_tables::*};
use crate::common::utils::{self, module_enabled, piece_value, Stats};
use crate::modules::search_extensions;
use crate::modules::skip_bad_moves;
use crate::modules::transposition_table::{self, TranspositionEntry};
use crate::modules::{alpha_beta, analyze};

use super::utils::Evaluation;

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
    pub(crate) naive_psqt_bishop_hash: HashMap<BitBoard, f32>,
}

struct NodeData {
    evaluation: Evaluation,
    debug_data: Option<Vec<String>>,
}

impl NodeData {
    fn new(evaluation: Evaluation, debug_data: Option<Vec<String>>) -> Self {
        NodeData {
            evaluation,
            debug_data,
        }
    }
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
        mut mg_incremental_psqt_eval: f32,
        mut eg_incremental_psqt_eval: f32,
    ) -> NodeData {
        if depth == 0 {
            stats.leaves_visited += 1;
            let eval = self.eval(
                board,
                board_played_times_prediction,
                mg_incremental_psqt_eval,
                eg_incremental_psqt_eval,
            );

            let evaluation = Evaluation::new(
                Some(eval),
                None,
                Some(mg_incremental_psqt_eval + eg_incremental_psqt_eval),
            );
            if module_enabled(self.modules, TRANSPOSITION_TABLE) {
                transposition_table::insert_in_transposition_table(
                    &mut self.transposition_table,
                    board,
                    depth,
                    stats,
                    evaluation,
                );
            }

            return NodeData::new(evaluation, None);
        }

        // Whether we should try to maximise the eval
        let maximise: bool = board.side_to_move() == Color::White;
        let mut best_evaluation = Evaluation::empty();

        let legal_moves = MoveGen::new_legal(board);
        let num_legal_moves = legal_moves.len();
        if num_legal_moves == 0 {
            best_evaluation.eval = Some(eval::eval_no_legal_moves(board));
            return NodeData::new(best_evaluation, None);
        }

        let transposition_table = if module_enabled(self.modules, TRANSPOSITION_TABLE) {
            Some(&self.transposition_table)
        } else {
            None
        };
        let mut boards = Self::create_board_list(board, stats, legal_moves, transposition_table);

        // Sort by eval
        Self::sort_by_eval(maximise, &mut boards);

        let mut debug_data = None;

        for (i, (chess_move, new_board, transposition_entry)) in boards.into_iter().enumerate() {
            if deadline.is_some_and(utils::passed_deadline) {
                // The previous value of progress_on_next_layer comes from deeper layers returning.
                // We want these contributions to be proportional to the contribution from a single
                // node on our layer
                stats.progress_on_next_layer *= 1. / num_legal_moves as f32;
                stats.progress_on_next_layer += i.saturating_sub(1) as f32 / num_legal_moves as f32;
                return NodeData::new(best_evaluation, None);
            };

            if depth > stats.max_depth {
                stats.max_depth = depth;
            }

            if module_enabled(self.modules, SKIP_BAD_MOVES)
                && skip_bad_moves::should_skip(num_legal_moves, i)
            {
                return NodeData::new(best_evaluation, None);
            }

            let search_extensions = module_enabled(self.modules, SEARCH_EXTENSIONS);
            let extend_by = search_extensions::calculate(
                num_extensions,
                num_legal_moves,
                new_board,
                search_extensions,
            );

            let evaluation = if let Some(transposition_entry) = transposition_entry {
                transposition_entry.evaluation
            } else {
                draw_checker::count_board(board_played_times_prediction, &new_board);
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
                    mg_incremental_psqt_eval,
                    eg_incremental_psqt_eval,
                );
                draw_checker::uncount_board(board_played_times_prediction, &new_board);
                debug_data = evaluation.debug_data;
                evaluation.evaluation
            };

            stats.nodes_visited += 1;

            // Replace best_eval if ours is better
            if eval::new_eval_is_better(maximise, &best_evaluation, &evaluation) {
                if original && module_enabled(self.modules, ANALYZE) {
                    debug_data = Some(analyze::get_debug_data(
                        self.modules,
                        maximise,
                        &best_evaluation,
                        &chess_move,
                        &evaluation,
                    ));
                }

                best_evaluation.eval = evaluation.eval;
                best_evaluation.next_action = Some(Action::MakeMove(chess_move));
            }

            if module_enabled(self.modules, ALPHA_BETA) {
                (alpha, beta) = alpha_beta::calc_new(alpha, beta, maximise, evaluation);
                if alpha > beta {
                    stats.alpha_beta_breaks += 1;
                    break;
                }
            }

            if module_enabled(self.modules, TAPERED_INCREMENTAL_PESTO_PSQT) {
                fn calc_increment(piece_type: Piece, location: usize, mg_eg: bool) -> f32 {
                    if mg_eg {
                        TAPERED_MG_PESTO[piece_type.to_index()][location]
                    } else {
                        TAPERED_EG_PESTO[piece_type.to_index()][location]
                    }
                }
                let moved_piece_type = board.piece_on(chess_move.get_source()).unwrap();

                let multiplier = if board.side_to_move() == Color::White {
                    1
                } else {
                    -1
                };
                let mut mg_incremental_psqt_eval_change = 0.;
                let mut eg_incremental_psqt_eval_change = 0.;
                if mg_incremental_psqt_eval_change == 0. || eg_incremental_psqt_eval_change == 0. {
                    for i in 0..5 {
                        mg_incremental_psqt_eval_change +=
                            Self::calc_tapered_psqt_eval(board, i, true);
                        mg_incremental_psqt_eval_change +=
                            Self::calc_tapered_psqt_eval(board, i, false);
                    }
                } else {
                    //Remove the eval from the previous square we stood on.
                    let source: usize = (56 - chess_move.get_source().to_int()
                        + 2 * (chess_move.get_source().to_int() % 8))
                        as usize;
                    mg_incremental_psqt_eval_change +=
                        calc_increment(moved_piece_type, source, true);
                    eg_incremental_psqt_eval_change +=
                        calc_increment(moved_piece_type, source, false);

                    //Increase the eval at the destination
                    let dest: usize = (56 - chess_move.get_dest().to_int()
                        + 2 * (chess_move.get_dest().to_int() % 8))
                        as usize;
                    mg_incremental_psqt_eval_change += calc_increment(moved_piece_type, dest, true);
                    eg_incremental_psqt_eval_change +=
                        calc_increment(moved_piece_type, dest, false);

                    // Decrement enemy eval from potential capture
                    if let Some(attacked_piece_type) = board.piece_on(chess_move.get_dest()) {
                        mg_incremental_psqt_eval_change +=
                            calc_increment(attacked_piece_type, dest, true);
                        eg_incremental_psqt_eval_change +=
                            calc_increment(attacked_piece_type, dest, false);
                    }
                }
                mg_incremental_psqt_eval += mg_incremental_psqt_eval_change * multiplier as f32;
                eg_incremental_psqt_eval += eg_incremental_psqt_eval_change * multiplier as f32;
            }
            best_evaluation.incremental_psqt_eval =
                Some(mg_incremental_psqt_eval + eg_incremental_psqt_eval);
        }

        if module_enabled(self.modules, TRANSPOSITION_TABLE) && depth >= 3 {
            transposition_table::insert_in_transposition_table(
                &mut self.transposition_table,
                board,
                depth,
                stats,
                best_evaluation,
            );
        }

        if debug_data.is_some() {
            let mut debug_data_unwrapped = debug_data.unwrap();
            if let Some(Action::MakeMove(next_move)) = best_evaluation.next_action {
                utils::vector_push_debug!(
                    debug_data_unwrapped,
                    best_evaluation.eval,
                    next_move.to_string(),
                );
            }
            debug_data = Some(debug_data_unwrapped);
        }
        NodeData::new(best_evaluation, debug_data)
    }

    fn create_board_list(
        board: &Board,
        stats: &mut Stats,
        legal_moves: MoveGen,
        transposition_table: Option<&HashMap<Board, TranspositionEntry>>,
    ) -> Vec<(ChessMove, Board, Option<TranspositionEntry>)> {
        legal_moves
            .map(|chess_move| {
                let board = board.make_move_new(chess_move);
                let mut transposition_entry = None;
                if let Some(transposition_table) = transposition_table {
                    transposition_entry = transposition_table::get_transposition_entry(
                        transposition_table,
                        stats,
                        &board,
                    );
                }
                (chess_move, board, transposition_entry)
            })
            .collect::<Vec<(ChessMove, Board, Option<TranspositionEntry>)>>()
    }

    fn sort_by_eval(maximise: bool, boards: &mut [(ChessMove, Board, Option<TranspositionEntry>)]) {
        boards.sort_by(|board1, board2| {
            let eval1 = if let Some(entry) = board1.2 {
                entry.evaluation.eval.unwrap_or_default()
            } else {
                0.
            };
            let eval2 = if let Some(entry) = board2.2 {
                entry.evaluation.eval.unwrap_or_default()
            } else {
                0.
            };
            let ordering = eval1.partial_cmp(&eval2).expect("Eval is a valid value");

            if maximise {
                return ordering.reverse();
            }
            ordering
        });
    }

    fn next_action(
        &mut self,
        board: &Board,
        depth: u32,
        deadline: Option<Instant>,
    ) -> (Option<Action>, Vec<String>, Stats) {
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
            0.,
            0.,
        );
        let analyzer_data = out.debug_data.unwrap_or_default();
        (out.evaluation.next_action, analyzer_data, stats)
    }

    pub(crate) fn next_action_iterative_deepening(
        &mut self,
        board: &Board,
        deadline: Instant,
    ) -> (Action, Vec<String>, Stats) {
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
                // We should declare draw by three-fold repetition. This is not checked
                // unless we do this.
                action = Action::DeclareDraw;
            }
            self.board_played_times.insert(new_board, old_value + 1);
        }

        (action, deepest_complete_output.1, deepest_complete_output.2)
    }

    pub(crate) fn eval(
        &mut self,
        board: &Board,
        board_played_times_prediction: &HashMap<Board, u32>,
        mg_incremental_psqt_eval: f32,
        eg_incremental_psqt_eval: f32,
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
        let material_each_side: (u32, u32) = utils::material_each_side(board);

        // Negative when black has advantage
        let diff_material: i32 = material_each_side.0 as i32 - material_each_side.1 as i32;

        let mut controlled_squares = 0;
        if module_enabled(self.modules, SQUARE_CONTROL_METRIC) {
            controlled_squares = if board.side_to_move() == Color::Black {
                -1i32
            } else {
                1i32
            } * MoveGen::new_legal(board).count() as i32;
        }

        // Compares piece position with an 8x8 table containing certain values. The value corresponding to the position of the piece gets added as evaluation.
        let mut naive_psqt: f32 = 0.;
        if module_enabled(self.modules, NAIVE_PSQT) {
            fn naive_psqt_calc(
                naive_psqt_table: [f32; 64],
                piece_bitboard: &BitBoard,
                color_bitboard: &BitBoard,
            ) -> f32 {
                // Essentially, gets the dot product between a "vector" of the bitboard (containing 64 0s and 1s) and the table with NAIVE_PSQT bonus constants.
                let mut bonus: f32 = 0.;
                // Gets the bitboard with all piece NAIVE_PSQTs, and runs bitwise and for the board having one's own colors.
                for (i, table_entry) in naive_psqt_table.iter().enumerate() {
                    //The naive_psqt table and bitboard are flipped vertically, hence .reverse_colors(). Reverse colors is for some reason faster than replacing i with 56-i+2*(i%8).
                    bonus += ((piece_bitboard & color_bitboard)
                        .reverse_colors()
                        .to_size(i as u8)
                        & 1) as f32
                        * table_entry;
                }
                bonus
            }

            macro_rules! in_hash_map {
                ($board: tt, $piece: tt, $table: tt, $hashmap: tt) => {
                    in_hash_map(
                        $board.pieces(Piece::$piece),
                        $board.color_combined($board.side_to_move()),
                        $table,
                        &mut self.$hashmap,
                    )
                };
            }

            /// Utilizes hashmaps so that we don't have to recalculate the entire bonus for all pieces every move. This is slightly faster.
            /// Either calculates native_psqt or takes it from the hashmap if it exists
            fn in_hash_map(
                piece_bitboard: &BitBoard,
                color_bitboard: &BitBoard,
                naive_psqt_table: [f32; 64],
                naive_psqt_hash_map: &mut HashMap<BitBoard, f32>,
            ) -> f32 {
                *naive_psqt_hash_map
                    .entry(piece_bitboard & color_bitboard)
                    .or_insert_with(|| {
                        naive_psqt_calc(naive_psqt_table, piece_bitboard, color_bitboard)
                    })
            }

            naive_psqt += in_hash_map!(board, Pawn, NAIVE_PSQT_TABLE_PAWN, naive_psqt_pawn_hash);
            naive_psqt += in_hash_map!(board, Rook, NAIVE_PSQT_TABLE_ROOK, naive_psqt_rook_hash);
            naive_psqt += in_hash_map!(board, King, NAIVE_PSQT_TABLE_KING, naive_psqt_king_hash);
            naive_psqt += in_hash_map!(board, Queen, NAIVE_PSQT_TABLE_QUEEN, naive_psqt_queen_hash);
            naive_psqt += in_hash_map!(
                board,
                Bishop,
                NAIVE_PSQT_TABLE_BISHOP,
                naive_psqt_bishop_hash
            );
            naive_psqt += in_hash_map!(
                board,
                Knight,
                NAIVE_PSQT_TABLE_KNIGHT,
                naive_psqt_knight_hash
            );
        }

        let mut mg_tapered_pesto: f32 = 0.;
        let mut eg_tapered_pesto: f32 = 0.;
        let mut tapered_pesto: f32 = 0.;
        if module_enabled(self.modules, TAPERED_EVERY_PESTO_PSQT) {
            for i in 0..5 {
                mg_tapered_pesto += Self::calc_tapered_psqt_eval(board, i, true);
                eg_tapered_pesto += Self::calc_tapered_psqt_eval(board, i, false);
            }
            tapered_pesto = ((material_each_side.0 + material_each_side.1
                - 2 * piece_value(Piece::King)) as f32
                * mg_tapered_pesto
                + (78
                    - (material_each_side.0 + material_each_side.1 - 2 * piece_value(Piece::King)))
                    as f32
                    * eg_tapered_pesto)
                / 78.;
        }

        let mut pawn_structure: f32 = 0.;
        if module_enabled(self.modules, PAWN_STRUCTURE) {
            fn pawn_structure_calc(
                all_pawn_bitboard: &BitBoard,
                color_bitboard: &BitBoard,
                all_king_bitboard: &BitBoard,
            ) -> f32 {
                let mut bonus: f32 = 0.;
                let pawn_bitboard: usize = (all_pawn_bitboard & color_bitboard).to_size(0);
                let king_bitboard: usize = (all_king_bitboard & color_bitboard).to_size(0);
                //pawn chain, awarding 0.5 eval for each pawn protected by another pawn. Constants should in theory cover a (literal) edge case... I hope.
                bonus += 0.5
                    * ((pawn_bitboard & 0xFEFEFEFEFEFEFEFE & (pawn_bitboard << 9)).count_ones()
                        + (pawn_bitboard & 0x7F7F7F7F7F7F7F7F & (pawn_bitboard << 7)).count_ones())
                        as f32;

                //stacked pawns. -0.5 points per rank containing >1 pawns. By taking the pawn bitboard and operating bitwise AND for another bitboard (integer) where the leftmost rank is filled. This returns all pawns in that rank. By bitshifting we can choose rank. Additionally by counting we get number of pawns. We then remove 1 as we only want to know if there are >1 pawn. If there is, subtract 0.5 points per extra pawn.
                for i in 0..7 {
                    //constant 0x8080808080808080: entire first rank.
                    bonus -= 0.5
                        * ((pawn_bitboard & (0x8080808080808080 >> i)).count_ones() as f32 - 1.)
                            .max(0.);
                }

                //king safety. Outer 3 pawns get +1 eval bonus per pawn if king is behind them. King bitboard required is either ..X..... or ......X.
                bonus += ((king_bitboard & 0x40).count_ones()
                    * (pawn_bitboard & 0x80E000).count_ones()
                    + (king_bitboard & 0x4).count_ones() * (pawn_bitboard & 0x1070000).count_ones())
                    as f32;
                bonus
            }

            //Because pawn moves (according to chessprogramming.org) are rarely performed, hashing them is useful.
            let pawn_bitboard: BitBoard =
                board.pieces(Piece::Pawn) & board.color_combined(board.side_to_move());
            self.pawn_hash.entry(pawn_bitboard).or_insert_with(|| {
                pawn_structure_calc(
                    board.pieces(Piece::Pawn),
                    board.color_combined(board.side_to_move()),
                    board.pieces(Piece::King),
                )
            });
            pawn_structure = *self.pawn_hash.get(&pawn_bitboard).unwrap();
        }

        let mut incremental_psqt_eval: f32 = 0.;
        if module_enabled(self.modules, TAPERED_INCREMENTAL_PESTO_PSQT) {
            incremental_psqt_eval = (material_each_side.0 + material_each_side.1
                - 2 * piece_value(Piece::King)) as f32
                * mg_incremental_psqt_eval
                + (78 - material_each_side.0 + material_each_side.1 - 2 * piece_value(Piece::King))
                    as f32
                    * eg_incremental_psqt_eval
        }

        let evaluation: f32 = controlled_squares as f32 / 20.
            + diff_material as f32
            + naive_psqt
            + pawn_structure
            + tapered_pesto
            + incremental_psqt_eval;
        evaluation
    }

    fn calc_tapered_psqt_eval(board: &Board, piece: u8, mg_eg: bool) -> f32 {
        fn tapered_psqt_calc(
            piece_bitboard: &BitBoard,
            color_bitboard: &BitBoard,
            piece_index: usize,
            mg_eg: bool,
        ) -> f32 {
            // Essentially, gets the dot product between a "vector" of the bitboard (containing 64 0s and 1s) and the table with NAIVE_PSQT bonus constants.
            let mut bonus: f32 = 0.;

            if mg_eg {
                // Gets the bitboard with all piece positions, and runs bitwise and for the board having one's own colors.
                // Iterates over all 64 squares on the board.
                for i in 0..63 {
                    // The psqt tables and bitboards are flipped vertically, hence .reverse_colors().
                    // Reverse colors is for some reason faster than replacing i with 56-i+2*(i%8).
                    // By being tapered, it means that we have an (opening + middlegame) and an endgame PSQT,
                    // and we (hopefully?) linerarly transition from one to the other, depending on material value.
                    bonus += ((piece_bitboard & color_bitboard)
                        .reverse_colors()
                        .to_size(i as u8)
                        & 1) as f32
                        * TAPERED_MG_PESTO[piece_index][i];
                }
                bonus
            } else {
                for i in 0..63 {
                    bonus += ((piece_bitboard & color_bitboard)
                        .reverse_colors()
                        .to_size(i as u8)
                        & 1) as f32
                        * TAPERED_EG_PESTO[piece_index][i];
                }
                bonus
            }
        }

        macro_rules! tapered_psqt_calc {
            ($board: tt, $piece: tt, $index: tt, $mg_eg: tt) => {
                tapered_psqt_calc(
                    $board.pieces(Piece::$piece),
                    $board.color_combined($board.side_to_move()),
                    $index,
                    $mg_eg,
                )
            };
        }
        match piece {
            0 => tapered_psqt_calc!(board, Pawn, 0, mg_eg),
            1 => tapered_psqt_calc!(board, Knight, 1, mg_eg),
            2 => tapered_psqt_calc!(board, Bishop, 2, mg_eg),
            3 => tapered_psqt_calc!(board, Rook, 3, mg_eg),
            4 => tapered_psqt_calc!(board, Queen, 4, mg_eg),
            5 => tapered_psqt_calc!(board, King, 5, mg_eg),
            6_u8..=u8::MAX => unimplemented!(),
        }
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
