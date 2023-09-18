use chess::{ChessMove, Game, MoveGen};
use rand::Rng;

pub(crate) fn random_starting_position(num_random_moves: u32) -> Game {
    let mut game = Game::new();
    for _ in 0..num_random_moves {
        let board = game.current_position();
        let legal_moves: Vec<ChessMove> = MoveGen::new_legal(&board).collect();

        game.make_move(legal_moves[rand::thread_rng().gen_range(0..legal_moves.len())]);
    }
    game
}
