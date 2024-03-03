use std::mem;
use std::sync::Arc;
use tokio::time::{Duration, Instant};

use chess::{Action, Board, Color, Game, GameResult};
use tokio::sync::Mutex;

use crate::algorithms::the_algorithm::Algorithm;
use crate::common::constants::modules::ANALYZE;
use crate::common::utils::{self, Stats};
use crate::PRINT_GAME;

pub(crate) struct Competition {
    pub(crate) algo1: Algorithm,
    pub(crate) algo2: Algorithm,
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
            (InconclusiveTooLong, _) => GamePairOutcome::InconclusiveTooLong,
            (_, InconclusiveTooLong) => GamePairOutcome::InconclusiveTooLong,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Default)]
pub(crate) enum GameOutcome {
    WhiteWin,
    BlackWin,
    Draw,
    #[default]
    InconclusiveTooLong,
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
    /// First is algo1 stats and second is algo2 stats
    stats: (Stats, Stats),

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
    pub(crate) fn new(algo1: Algorithm, algo2: Algorithm) -> Competition {
        Self {
            algo1,
            algo2,
            results: None,
        }
    }

    pub(crate) fn play_game(&self, mut game: Game, reversed: bool, max_plies: usize) -> GameInfo {
        let mut game_info = GameInfo::default();
        let mut algo1 = self.algo1.clone();
        algo1.reset();
        let mut algo2 = self.algo2.clone();
        algo2.reset();
        if reversed {
            mem::swap(&mut algo1, &mut algo2);
        };

        let mut num_plies = 0;
        loop {
            let start = Instant::now();
            let side_to_move = game.side_to_move();
            let analyze: bool;
            let mut next_action = match side_to_move {
                Color::White => {
                    analyze = algo1.modules & ANALYZE != 0;
                    algo1.next_action_iterative_deepening(
                        &game.current_position(),
                        Instant::now() + algo1.time_per_move,
                    )
                }
                Color::Black => {
                    analyze = algo1.modules & ANALYZE != 0;
                    algo2.next_action_iterative_deepening(
                        &game.current_position(),
                        Instant::now() + algo2.time_per_move,
                    )
                }
            };
            let end = Instant::now();
            next_action.2.time_spent = end - start;
            next_action.2.num_plies = 1;

            if analyze {
                // Add stats field to the debug thing
                utils::vector_push_debug!(next_action.1, next_action.2);
            }

            if side_to_move == Color::Black && !reversed || side_to_move == Color::White && reversed
            {
                // This means algo2 is playing
                // next_action.2 is has the Stats object. TODO: Make this clearer by refactoring
                game_info.stats.1 += next_action.2;
            } else {
                game_info.stats.0 += next_action.2;
            }

            let mut declared_draw = false;
            let success = match next_action.0 {
                Action::MakeMove(chess_move) => game.make_move(chess_move),
                Action::OfferDraw(color) => game.offer_draw(color),
                Action::AcceptDraw => game.accept_draw(),
                Action::DeclareDraw => {
                    // The chess crate one is really bad and wrong
                    declared_draw = true;
                    true
                }
                Action::Resign(color) => game.resign(color),
            };

            if !success {
                dbg!(utils::to_pgn(&game));
                panic!("Algorithm made illegal action");
            }

            if declared_draw {
                game_info.outcome = GameOutcome::Draw;
                break;
            }

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
                game_info.outcome = GameOutcome::InconclusiveTooLong;
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

    pub(crate) async fn start_competition(self, num_game_pairs: u32) -> CompetitionResults {
        if let Some(results) = self.results {
            return results;
        }
        let results = Arc::new(Mutex::new(CompetitionResults::default()));
        let self_arc = Arc::new(self);

        let sum_stats = Arc::new(Mutex::new((Stats::default(), Stats::default())));

        let mut tasks = Vec::new();
        for _ in 0..num_game_pairs {
            let results = results.clone();
            let sum_stats = sum_stats.clone();
            let self_arc = self_arc.clone();
            let task = tokio::spawn(async move {
                let game = utils::random_starting_position(5);

                let game_pair_info = self_arc.play_game_pair(game);
                let combined_outcome = GamePairOutcome::combine_outcomes(
                    game_pair_info.0.outcome,
                    game_pair_info.1.outcome,
                );

                //Whether the game just played should be printed in console.
                if PRINT_GAME {
                    println!("Game pair played.  Outcome: {:?}", combined_outcome);
                    println!("{}", utils::to_pgn(&game_pair_info.0.game.unwrap()));
                }

                results.lock().await.register_game_outcome(combined_outcome);

                let mut locked_stats = sum_stats.lock().await;
                // First game algo1
                locked_stats.0 += game_pair_info.0.stats.0;
                // Second game algo1
                locked_stats.0 += game_pair_info.1.stats.0;
                // First game algo2
                locked_stats.1 += game_pair_info.0.stats.1;
                // Second game algo2
                locked_stats.1 += game_pair_info.1.stats.1;
            });
            tasks.push(task);
        }
        for task in tasks {
            task.await;
        }
        let sum_stats = sum_stats.lock().await;
        let avg_stats = (
            sum_stats.0 / sum_stats.0.num_plies,
            sum_stats.1 / sum_stats.1.num_plies,
        );

        println!("Stats for algo1: {:#?}", avg_stats.0);
        println!("Stats for algo2: {:#?}", avg_stats.1);

        // Gives E0597 otherwise
        #[allow(clippy::let_and_return)]
        let results_copy = *results.lock().await;
        results_copy
    }

    // #[allow(dead_code)]
    // fn get_average_eval(&self, game: &Game) -> f32 {
    //     let board = game.current_position();
    //     self.algo1.eval(&board) + self.algo2.eval(&board) / 2.
    // }

    #[allow(dead_code)]
    pub(crate) fn find_game<P>(&mut self, predicate: P) -> Option<(GameInfo, GameInfo)>
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
            self.algo1.reset();
            self.algo2.reset();
        }
    }

    pub(crate) fn analyze_algorithm_choices<P>(&mut self, predicate: P)
    where
        P: Fn(&(GameInfo, GameInfo), GamePairOutcome) -> bool,
    {
        let game = self.find_game(predicate);

        let game = game.unwrap();

        let mut i = 1;
        println!("{}", utils::to_pgn(game.0.game.as_ref().unwrap()));
        let mut board = Board::default();

        self.algo1.reset();
        self.algo2.reset();
        for chess_move in game.0.game.as_ref().unwrap().actions() {
            let Action::MakeMove(chess_move) = chess_move else {continue};

            let start = Instant::now();
            let mut algo_out = if i % 2 == 1 {
                // White's turn
                self.algo1.next_action_iterative_deepening(
                    &board,
                    Instant::now() + Duration::from_micros(2000),
                )
            } else {
                self.algo2.next_action_iterative_deepening(
                    &board,
                    Instant::now() + Duration::from_micros(2000),
                )
            };
            let end = Instant::now();
            algo_out.2.time_spent = end - start;
            // Add stats field to the debug thing
            utils::vector_push_debug!(algo_out.1, algo_out.2);

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

        dbg!(&self.algo1.board_played_times.values());
        dbg!(&self.algo2.board_played_times.values());
    }
}
