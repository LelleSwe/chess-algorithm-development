use std::collections::HashMap;

use chess::Board;

pub fn uncount_board(board_played_times_prediction: &mut HashMap<Board, u32>, new_board: &Board) {
    board_played_times_prediction.insert(
        // TODO Hash it to avoid copying, we need a good hash function for Board
        *new_board,
        *board_played_times_prediction.get(new_board).unwrap_or(&0) - 1,
    );
}

pub fn count_board(board_played_times_prediction: &mut HashMap<Board, u32>, new_board: &Board) {
    board_played_times_prediction.insert(
        // TODO Hash it to avoid copying, we need a good hash function for Board
        *new_board,
        *board_played_times_prediction.get(new_board).unwrap_or(&0) + 1,
    );
}
