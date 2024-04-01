use chess::Board;

pub fn calculate(
    num_extensions: u32,
    num_legal_moves: usize,
    new_board: Board,
) -> u32 {
    if num_extensions > 3 {
        0
    } else if num_legal_moves <= 3 || new_board.checkers().popcnt() >= 2 {
        1
    } else {
        0
    }
}
