use std::time::Duration;

use crate::algorithms::the_algorithm::Algorithm;
use crate::common::constants::modules::{
    ALPHA_BETA, ANALYZE, SEARCH_EXTENSIONS, SKIP_BAD_MOVES, SQUARE_CONTROL_METRIC,
    TRANSPOSITION_TABLE, POSITION_BONUS
};

use self::pitter::logic::{Competition, GameOutcome};

mod algorithms;
mod common;
mod pitter;

#[tokio::main]
async fn main() {
    let modules1 = ALPHA_BETA | POSITION_BONUS;
    let modules2 = ALPHA_BETA;
    let time_per_move1 = Duration::from_micros(500);
    let time_per_move2 = Duration::from_micros(500);

    let competition = Competition::new(
        Algorithm::new(modules1, time_per_move1),
        Algorithm::new(modules2, time_per_move2),
    );

    // competition.analyze_algorithm_choices(|(game_info, _), _| {
    //     game_info.outcome == GameOutcome::InconclusiveTooLong
    // });
    let results = competition.start_competition(2000).await;
    dbg!(results);
}
