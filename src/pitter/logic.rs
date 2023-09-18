use chess::{Board, ChessMove, Game};

use crate::common::algorithm::Algorithm;
use crate::common::utils;

struct Competition {
    algo1: Box<dyn Algorithm>,
    algo2: Box<dyn Algorithm>,
    game_history: Vec<Board>,
    results: Option<CompetitionResults>,
}

enum GameOutcome {
    WhiteWin,
    /// White won one and when the algorithms switched places it was a draw
    WhiteHalfWin,
    BlackWin,
    /// Black won one and when the algorithms switched places it was a draw
    BlackHalfWin,
    Draw,
    Inconclusive,
}

#[derive(Default, Debug, Copy, Clone)]
struct CompetitionResults {
    /// How many pairs of games that algo1 wins from algoh positions
    algo1_wins: usize,
    /// How many pairs of games that algo2 wins from algoh positions
    algo2_wins: usize,
    /// Pairs of games that draw no matter which algo is playing which side
    draws: usize,
    /// Same color wins no matter what algo is playing it
    inconclusive: usize,
}

impl CompetitionResults {
    /// Reversed == true means that algo1 played black
    fn register_game_outcome(&mut self, game_outcome: GameOutcome, reversed: bool) {
        match game_outcome {
            GameOutcome::WhiteWin => {
                if reversed {
                    self.algo2_wins += 1
                } else {
                    self.algo1_wins += 1
                }
            }
            GameOutcome::BlackWin => {
                if reversed {
                    self.algo1_wins += 1
                } else {
                    self.algo2_wins += 1
                }
            }
            GameOutcome::Draw => self.draws += 1,
            GameOutcome::Inconclusive => self.inconclusive += 1,
            GameOutcome::WhiteHalfWin => todo!(),
            GameOutcome::BlackHalfWin => todo!(),
        }
    }
}

/// Reversed == true means that algo1 plays black
impl Competition {
    fn new(algo1: Box<dyn Algorithm>, algo2: Box<dyn Algorithm>) -> Competition {
        Self {
            algo1,
            algo2,
            game_history: Vec::new(),
            results: None,
        }
    }

    fn play_game(&self, game: &Game, reversed: bool) -> GameOutcome {
        todo!();
    }

    fn start_competition(&mut self) -> CompetitionResults {
        if let Some(results) = self.results {
            return results;
        }
        let mut results = CompetitionResults::default();
        let game = utils::random_starting_position(5);

        for _ in 0..500 {
            for reversed in 0..=1 {
                let reversed = reversed % 2 == 1;
                let outcome = self.play_game(&game, reversed);
                results.register_game_outcome(outcome, reversed);
            }
        }
        self.results = Some(results);
        results
    }

    fn get_average_eval(&self, board: Board) -> f32 {
        todo!();
    }
}
