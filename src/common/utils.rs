use std::ops::{AddAssign, Div};
use tokio::time::{Duration, Instant};

use chess::{Board, ChessMove, Color, Game, MoveGen, Piece};
use rand::Rng;

pub(crate) fn random_starting_position(num_random_moves: u32) -> Game {
    let mut game = Game::new();
    for _ in 0..num_random_moves {
        let board = game.current_position();
        let legal_moves: Vec<ChessMove> = MoveGen::new_legal(&board).collect();

        if legal_moves.is_empty() {
            return random_starting_position(num_random_moves);
        }
        game.make_move(legal_moves[rand::thread_rng().gen_range(0..legal_moves.len())]);
    }
    game
}

/// Returns tuple where first element is white total material and last element is black total
/// material
pub(crate) fn material_each_side(board: &Board) -> (u32, u32) {
    let mut output = (0, 0);
    for piece_type in chess::ALL_PIECES {
        let piece_bitboard = board.pieces(piece_type);
        let bitboard_white = board.color_combined(Color::White) & piece_bitboard;
        let num_white_pieces = bitboard_white.popcnt();
        output.0 += num_white_pieces * piece_value(piece_type);

        // Black pieces are equal to the total amount minus the white pieces
        let num_black_pieces = piece_bitboard.popcnt() - num_white_pieces;
        output.1 += num_black_pieces * piece_value(piece_type);
    }
    output
}

pub(crate) fn piece_value(piece: Piece) -> u32 {
    match piece {
        Piece::Pawn => 1,
        Piece::Knight => 3,
        Piece::Bishop => 3,
        Piece::Rook => 5,
        Piece::Queen => 9,
        Piece::King => 1000,
    }
}

pub(crate) fn to_pgn(game: &Game) -> String {
    let mut output = String::new();
    let mut i = 0;
    for action in game.actions() {
        if let chess::Action::MakeMove(chess_move) = action {
            if i != 0 {
                output.push(' ');
            }
            if i % 2 == 0 {
                output.push_str(&format!("{}. ", i / 2 + 1))
            }
            output.push_str(&chess_move.to_string());
            i += 1;
        }
    }
    output
}

/// Pushes the debug string representation into this vector. Used for printing debug information
macro_rules! vector_push_debug {
    ($vec:expr, $var:expr $(,)?) => {
        $vec.push(format!("{} = {:?}", stringify!($var), $var))
    };
    ($vec:expr, $($var:expr),+ $(,)?) =>{
        ($($crate::common::utils::vector_push_debug!($vec, $var)),+,)
    }
}

pub(crate) use vector_push_debug;

#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct Stats {
    pub(crate) alpha_beta_breaks: u32,
    pub(crate) depth: u32,
    pub(crate) max_depth: u32,
    pub(crate) leaves_visited: u32,
    pub(crate) nodes_visited: u32,
    pub(crate) num_plies: u32,
    pub(crate) time_spent: Duration,
    pub(crate) progress_on_next_layer: f32,
    pub(crate) transposition_table_entries: u32,
    pub(crate) transposition_table_accesses: u32,
    pub(crate) time_for_transposition_access: Duration,
}

impl AddAssign for Stats {
    fn add_assign(&mut self, rhs: Self) {
        self.nodes_visited += rhs.nodes_visited;
        self.depth += rhs.depth;
        self.max_depth += rhs.max_depth;
        self.leaves_visited += rhs.leaves_visited;
        self.alpha_beta_breaks += rhs.alpha_beta_breaks;
        self.num_plies += rhs.num_plies;
        self.time_spent += rhs.time_spent;
        self.progress_on_next_layer += rhs.progress_on_next_layer;
        self.transposition_table_entries += rhs.transposition_table_entries;
        self.transposition_table_accesses += rhs.transposition_table_accesses;
        self.time_for_transposition_access += rhs.time_for_transposition_access;
    }
}

impl Div<u32> for Stats {
    type Output = StatsAverage;

    fn div(self, rhs: u32) -> Self::Output {
        StatsAverage {
            alpha_beta_breaks: self.alpha_beta_breaks as f32 / rhs as f32,
            depth: self.depth as f32 / rhs as f32,
            max_depth: self.max_depth as f32 / rhs as f32,
            leaves_visited: self.leaves_visited as f32 / rhs as f32,
            nodes_visited: self.nodes_visited as f32 / rhs as f32,
            num_plies: self.num_plies as f32 / rhs as f32,
            time_spent: self.time_spent / rhs,
            progress_on_next_layer: self.progress_on_next_layer / rhs as f32,
            transposition_table_entries: self.transposition_table_entries as f32 / rhs as f32,
            transposition_table_accesses: self.transposition_table_accesses as f32 / rhs as f32,
            time_for_transposition_access: self.time_for_transposition_access / rhs,
        }
    }
}
#[derive(Default, Debug)]
// These fields are used through Debug
#[allow(dead_code)]
pub(crate) struct StatsAverage {
    pub(crate) alpha_beta_breaks: f32,
    pub(crate) depth: f32,
    pub(crate) max_depth: f32,
    pub(crate) leaves_visited: f32,
    pub(crate) nodes_visited: f32,
    pub(crate) num_plies: f32,
    pub(crate) time_spent: Duration,
    pub(crate) progress_on_next_layer: f32,
    pub(crate) transposition_table_entries: f32,
    pub(crate) transposition_table_accesses: f32,
    pub(crate) time_for_transposition_access: Duration,
}

pub(crate) fn passed_deadline(deadline: Instant) -> bool {
    let time_since_deadline = Instant::now().saturating_duration_since(deadline);
    !time_since_deadline.is_zero()
}

pub(crate) fn module_enabled(modules: u32, module_to_test: u32) -> bool {
    modules & module_to_test != 0
}