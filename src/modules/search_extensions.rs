use chess::Board;

pub fn calculate(
    num_extensions: u32,
    num_legal_moves: usize,
    new_board: Board,
    search_extensions: bool,
) -> u32 {
    let extend_by = if !search_extensions || num_extensions > 3 {
        0
    } else if num_legal_moves == 1 || new_board.checkers().popcnt() >= 2 {
        1
    } else {
        0
    };
    extend_by
}
