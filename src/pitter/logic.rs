use std::mem;
use std::time::{Duration, Instant};

use chess::{Action, Board, Color, Game, GameResult};

use crate::common::algorithm::Algorithm;
use crate::common::utils;

pub(crate) struct Competition {
    algo1: Box<dyn Algorithm>,
    algo2: Box<dyn Algorithm>,
    game_history: Vec<Board>,
    results: Option<CompetitionResults>,
}

#[derive(Debug, PartialEq, Eq)]
enum GamePairOutcome {
    Algo1Win,
    /// Algo1 won one and when the algorithms switched places it was a draw
    Algo1HalfWin,
    Algo2Win,
    /// Algo2 won one and when the algorithms switched places it was a draw
    Algo2HalfWin,
    Draw,
    Inconclusive,
}

impl GamePairOutcome {
    /// The first argument is the game where Algo1 played white, while the second is when they
    /// placed opposite sides of the board
    fn combine_outcomes(algo1_white: GameOutcome, algo2_white: GameOutcome) -> GamePairOutcome {
        use GameOutcome::*;
        use GamePairOutcome::{Algo1HalfWin, Algo1Win, Algo2HalfWin, Algo2Win};
        match (algo1_white, algo2_white) {
            (WhiteWin, WhiteWin) => GamePairOutcome::Inconclusive,
            (WhiteWin, BlackWin) => Algo1Win,
            (WhiteWin, Draw) => Algo1HalfWin,
            (BlackWin, WhiteWin) => Algo2Win,
            (BlackWin, BlackWin) => GamePairOutcome::Inconclusive,
            (BlackWin, Draw) => Algo2HalfWin,
            (Draw, WhiteWin) => Algo2HalfWin,
            (Draw, BlackWin) => Algo1HalfWin,
            (Draw, Draw) => GamePairOutcome::Draw,
            // Consider changing this in the future, this is an arbitrary choice
            (Inconclusive, _) => GamePairOutcome::Inconclusive,
            (_, Inconclusive) => GamePairOutcome::Inconclusive,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Default)]
enum GameOutcome {
    WhiteWin,
    BlackWin,
    Draw,
    #[default]
    Inconclusive,
}

#[derive(Default, Debug, Copy, Clone)]
pub(crate) struct CompetitionResults {
    /// How many pairs of games that Algo1 wins from both positions
    algo1_wins: usize,
    /// How many pairs of games that Algo2 wins from both positions
    algo2_wins: usize,
    /// Pairs of games that draw no matter which algo is playing which side
    draws: usize,
    /// Same color wins no matter what algo is playing it
    inconclusive: usize,

    /// Pairs of games that wins when Algo1 is playing and draw on the other
    algo1_half_wins: usize,
    /// Pairs of games that wins when Algo2 is playing and draw on the other
    algo2_half_wins: usize,
}

#[derive(Debug, Default)]
pub(crate) struct GameInfo {
    outcome: GameOutcome,
    num_plies_algo1: usize,
    num_plies_algo2: usize,

    time_spent_on_move_gen_algo1: Duration,
    time_spent_on_move_gen_algo2: Duration,
}

impl CompetitionResults {
    /// Reversed == true means that algo1 played black
    fn register_game_outcome(&mut self, game_outcome: GamePairOutcome) {
        match game_outcome {
            GamePairOutcome::Algo1Win => self.algo1_wins += 1,
            GamePairOutcome::Algo2Win => self.algo2_wins += 1,
            GamePairOutcome::Draw => self.draws += 1,
            GamePairOutcome::Inconclusive => self.inconclusive += 1,
            GamePairOutcome::Algo1HalfWin => self.algo1_half_wins += 1,
            GamePairOutcome::Algo2HalfWin => self.algo2_half_wins += 1,
        }
    }
}

/// Reversed == true means that algo1 plays black
impl Competition {
    pub(crate) fn new(algo1: Box<dyn Algorithm>, algo2: Box<dyn Algorithm>) -> Competition {
        Self {
            algo1,
            algo2,
            game_history: Vec::new(),
            results: None,
        }
    }

    fn play_game(&self, mut game: Game, reversed: bool, max_plies: usize) -> GameInfo {
        let mut game_info = GameInfo::default();
        let mut algo1 = &self.algo1;
        let mut algo2 = &self.algo2;
        if reversed {
            mem::swap(&mut algo1, &mut algo2);
        };

        let mut num_plies = 0;
        loop {
            let start = Instant::now();
            let side_to_move = game.side_to_move();
            let next_action = match side_to_move {
                Color::White => algo1.next_move(&game),
                Color::Black => algo2.next_move(&game),
            };
            let end = Instant::now();

            if side_to_move == Color::Black && !reversed || side_to_move == Color::White && reversed
            {
                game_info.time_spent_on_move_gen_algo2 += end - start;
                game_info.num_plies_algo2 += 1;
            } else {
                game_info.time_spent_on_move_gen_algo1 += end - start;
                game_info.num_plies_algo1 += 1;
            }

            match next_action {
                Action::MakeMove(chess_move) => game.make_move(chess_move),
                Action::OfferDraw(color) => game.offer_draw(color),
                Action::AcceptDraw => game.accept_draw(),
                Action::DeclareDraw => game.declare_draw(),
                Action::Resign(color) => game.resign(color),
            };

            if let Some(result) = game.result() {
                game_info.outcome = match result {
                    GameResult::WhiteCheckmates => GameOutcome::WhiteWin,
                    GameResult::WhiteResigns => GameOutcome::BlackWin,
                    GameResult::BlackCheckmates => GameOutcome::BlackWin,
                    GameResult::BlackResigns => GameOutcome::WhiteWin,
                    GameResult::Stalemate => GameOutcome::Draw,
                    GameResult::DrawAccepted => GameOutcome::Draw,
                    GameResult::DrawDeclared => GameOutcome::Draw,
                };
                return game_info;
            }
            if num_plies >= max_plies {
                game_info.outcome = GameOutcome::Inconclusive;
                return game_info;
            }
            num_plies += 1
        }
    }

    fn play_game_pair(&self, game: Game) -> (GameInfo, GameInfo) {
        let outcome1 = self.play_game(game.clone(), false, 500);
        let outcome2 = self.play_game(game, true, 500);

        (outcome1, outcome2)
    }

    pub(crate) fn start_competition(&mut self) -> CompetitionResults {
        if let Some(results) = self.results {
            return results;
        }
        let mut results = CompetitionResults::default();

        let mut time_spent_on_move_gen_algo1 = Duration::default();
        let mut time_spent_on_move_gen_algo2 = Duration::default();
        let mut num_plies_algo1 = 0;
        let mut num_plies_algo2 = 0;

        for _ in 0..500 {
            let game = utils::random_starting_position(5);
            let game_pair_info = self.play_game_pair(game);
            let combined_outcome = GamePairOutcome::combine_outcomes(
                game_pair_info.0.outcome,
                game_pair_info.1.outcome,
            );
            results.register_game_outcome(combined_outcome);

            time_spent_on_move_gen_algo1 += game_pair_info.0.time_spent_on_move_gen_algo1
                + game_pair_info.1.time_spent_on_move_gen_algo1;
            num_plies_algo1 += game_pair_info.0.num_plies_algo1 + game_pair_info.1.num_plies_algo1;

            time_spent_on_move_gen_algo2 += game_pair_info.0.time_spent_on_move_gen_algo2
                + game_pair_info.1.time_spent_on_move_gen_algo2;
            num_plies_algo2 += game_pair_info.0.num_plies_algo2 + game_pair_info.1.num_plies_algo2;
        }
        let time_per_move_algo1 = time_spent_on_move_gen_algo1 / num_plies_algo1 as u32;
        let time_per_move_algo2 = time_spent_on_move_gen_algo2 / num_plies_algo2 as u32;

        println!(
            "Finished! Outcomes: {:?}\nTime per move Algo1: {:?}\nTime per move Algo2: {:?}",
            results, time_per_move_algo1, time_per_move_algo2
        );
        self.results = Some(results);
        results
    }

    fn get_average_eval(&self, game: &Game) -> f32 {
        self.algo1.eval(game) + self.algo2.eval(game) / 2.
    }
}
