use std::mem;
use std::time::{Duration, Instant};

use chess::{Action, Board, Color, Game, GameResult};

use crate::common::algorithm::Algorithm;
use crate::common::utils;

pub(crate) struct Competition {
    pub(crate) algo1: Box<dyn Algorithm>,
    pub(crate) algo2: Box<dyn Algorithm>,
    results: Option<CompetitionResults>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GamePairOutcome {
    Algo1Win,
    /// Algo1 won one and when the algorithms switched places it was a draw
    Algo1HalfWin,
    Algo2Win,
    /// Algo2 won one and when the algorithms switched places it was a draw
    Algo2HalfWin,
    Draw,
    InconclusiveSameColorWin,
    InconclusiveTooLong,
}

impl GamePairOutcome {
    /// The first argument is the game where Algo1 played white, while the second is when they
    /// placed opposite sides of the board
    fn combine_outcomes(algo1_white: GameOutcome, algo2_white: GameOutcome) -> GamePairOutcome {
        use GameOutcome::*;
        use GamePairOutcome::{Algo1HalfWin, Algo1Win, Algo2HalfWin, Algo2Win};
        match (algo1_white, algo2_white) {
            (WhiteWin, WhiteWin) => GamePairOutcome::InconclusiveSameColorWin,
            (WhiteWin, BlackWin) => Algo1Win,
            (WhiteWin, Draw) => Algo1HalfWin,
            (BlackWin, WhiteWin) => Algo2Win,
            (BlackWin, BlackWin) => GamePairOutcome::InconclusiveSameColorWin,
            (BlackWin, Draw) => Algo2HalfWin,
            (Draw, WhiteWin) => Algo2HalfWin,
            (Draw, BlackWin) => Algo1HalfWin,
            (Draw, Draw) => GamePairOutcome::Draw,
            // Consider changing this in the future, this is an arbitrary choice
            (Inconclusive, _) => GamePairOutcome::InconclusiveTooLong,
            (_, Inconclusive) => GamePairOutcome::InconclusiveTooLong,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Default)]
pub(crate) enum GameOutcome {
    WhiteWin,
    BlackWin,
    Draw,
    #[default]
    Inconclusive,
}

#[allow(unused_assignments)]
#[derive(Default, Debug, Copy, Clone)]
pub(crate) struct CompetitionResults {
    /// How many pairs of games that Algo1 wins from both positions
    algo1_wins: usize,
    /// How many pairs of games that Algo2 wins from both positions
    algo2_wins: usize,
    /// Pairs of games that draw no matter which algo is playing which side
    draws: usize,
    /// Same color wins no matter what algo is playing it
    inconclusive_same_color_win: usize,

    /// One of the games went on for too long
    inconclusive_too_long: usize,

    /// Pairs of games that wins when Algo1 is playing and draw on the other
    algo1_half_wins: usize,
    /// Pairs of games that wins when Algo2 is playing and draw on the other
    algo2_half_wins: usize,
}

#[derive(Debug, Default)]
pub(crate) struct GameInfo {
    pub(crate) outcome: GameOutcome,
    num_plies_algo1: usize,
    num_plies_algo2: usize,

    time_spent_on_move_gen_algo1: Duration,
    time_spent_on_move_gen_algo2: Duration,

    pub(crate) game: Option<Game>,
}

impl CompetitionResults {
    /// Reversed == true means that algo1 played black
    fn register_game_outcome(&mut self, game_outcome: GamePairOutcome) {
        match game_outcome {
            GamePairOutcome::Algo1Win => self.algo1_wins += 1,
            GamePairOutcome::Algo2Win => self.algo2_wins += 1,
            GamePairOutcome::Draw => self.draws += 1,
            GamePairOutcome::InconclusiveTooLong => self.inconclusive_too_long += 1,
            GamePairOutcome::Algo1HalfWin => self.algo1_half_wins += 1,
            GamePairOutcome::InconclusiveSameColorWin => self.inconclusive_same_color_win += 1,
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
            results: None,
        }
    }

    pub(crate) fn play_game(&self, mut game: Game, reversed: bool, max_plies: usize) -> GameInfo {
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
                Color::White => algo1.next_action(
                    &game.current_position(),
                    false,
                    Instant::now() + Duration::from_micros(2000),
                ),
                Color::Black => algo2.next_action(
                    &game.current_position(),
                    false,
                    Instant::now() + Duration::from_micros(2000),
                ),
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

            match next_action.0 {
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
                break;
            }
            if num_plies >= max_plies {
                game_info.outcome = GameOutcome::Inconclusive;
                break;
            }
            num_plies += 1
        }

        game_info.game = Some(game);
        game_info
    }

    fn play_game_pair(&self, game: Game) -> (GameInfo, GameInfo) {
        let outcome1 = self.play_game(game.clone(), false, 150);
        let outcome2 = self.play_game(game, true, 150);

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

        let mut first_half_win = true;
        for _ in 0..200 {
            let game = utils::random_starting_position(5);

            let game_pair_info = self.play_game_pair(game);
            let combined_outcome = GamePairOutcome::combine_outcomes(
                game_pair_info.0.outcome,
                game_pair_info.1.outcome,
            );
            if combined_outcome == GamePairOutcome::Algo2HalfWin && first_half_win {
                first_half_win = false;
                println!("{}", utils::to_pgn(&game_pair_info.0.game.unwrap()));
                println!("{}", utils::to_pgn(&game_pair_info.1.game.unwrap()));
            }
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
            "Time per move Algo1: {:?}\nTime per move Algo2: {:?}",
            time_per_move_algo1, time_per_move_algo2
        );
        self.results = Some(results);
        results
    }

    #[allow(dead_code)]
    fn get_average_eval(&self, game: &Game) -> f32 {
        let board = game.current_position();
        self.algo1.eval(&board) + self.algo2.eval(&board) / 2.
    }

    pub(crate) fn find_game<P>(&self, predicate: P) -> Option<(GameInfo, GameInfo)>
    where
        P: Fn(&(GameInfo, GameInfo), GamePairOutcome) -> bool,
    {
        let mut i = 0;
        loop {
            let game = utils::random_starting_position((i % 100) * 2 + 4);

            let game_pair_info = self.play_game_pair(game);
            let combined_outcome = GamePairOutcome::combine_outcomes(
                game_pair_info.0.outcome,
                game_pair_info.1.outcome,
            );

            if predicate(&game_pair_info, combined_outcome) {
                return Some(game_pair_info);
            }
            i += 1;
            if i > 500 {
                return None;
            }
        }
    }

    pub(crate) fn analyze_algorithm_choices<P>(&self, predicate: P)
    where
        P: Fn(&(GameInfo, GameInfo), GamePairOutcome) -> bool,
    {
        let game = self.find_game(predicate);

        let game = game.unwrap();

        let mut i = 1;
        println!("{}", utils::to_pgn(game.0.game.as_ref().unwrap()));
        let mut board = Board::default();

        for chess_move in game.0.game.as_ref().unwrap().actions() {
            let Action::MakeMove(chess_move) = chess_move else {continue};

            // PROBLEM!! This will be unreliable if the algorithms have persistent data over moves
            // in the future, we will have to make new instances of the algos somehow then
            let algo_out = if i % 2 == 1 {
                // White's turn
                self.algo1
                    .next_action(&board, true, Instant::now() + Duration::from_micros(2000))
            } else {
                self.algo2
                    .next_action(&board, true, Instant::now() + Duration::from_micros(2000))
            };

            if i % 2 == 1 {
                println!("{}. {} ...", (i + 1) / 2, chess_move);
            } else {
                println!("{}. ... {}", (i + 1) / 2, chess_move);
            }
            for analyze_string in algo_out.1 {
                println!("  - {}", analyze_string);
            }
            board = board.make_move_new(*chess_move);
            i += 1;
        }
    }
}
