use std::collections::HashMap;

use chess::Board;

pub fn uncount_board(board_played_times_prediction: &mut HashMap<u64, u32>, new_board: &Board) {
    let hash = new_board.get_hash();
    board_played_times_prediction.insert(
        // TODO Hash it to avoid copying, we need a good hash function for Board
        hash,
        *board_played_times_prediction.get(&hash).unwrap_or(&0) - 1,
    );
}

pub fn count_board(board_played_times_prediction: &mut HashMap<u64, u32>, new_board: &Board) {
    let hash = new_board.get_hash();
    board_played_times_prediction.insert(
        hash,
        *board_played_times_prediction.get(&hash).unwrap_or(&0) + 1,
    );
}
