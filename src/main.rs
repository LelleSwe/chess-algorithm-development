use std::time::Duration;

use crate::algorithms::the_algorithm::Algorithm;
#[allow(unused_imports)]
use crate::common::constants::modules::{
    ALPHA_BETA, ANALYZE, SEARCH_EXTENSIONS, SKIP_BAD_MOVES, SQUARE_CONTROL_METRIC,
    TRANSPOSITION_TABLE, NAIVE_PSQT, PAWN_STRUCTURE, TAPERED_EVERY_PRESTO_PSQT, TAPERED_INCREMENTAL_PESTO_PSQT
};

use self::pitter::logic::Competition;

mod algorithms;
mod common;
mod pitter;

#[tokio::main]
async fn main() {
    //ALPHA_BETA | ANALYZE | SEARCH_EXTENSIONS | SKIP_BAD_MOVES | SQUARE_CONTROL_METRIC | TRANSPOSITION_TABLE | NAIVE_PSQT | PAWN_STRUCTURE | TAPERED_EVERY_PRESTO_PSQT | TAPERED_INCREMENTAL_PESTO_PSQT
    let modules1 = ALPHA_BETA | TAPERED_EVERY_PRESTO_PSQT;
    let modules2 = ALPHA_BETA | NAIVE_PSQT;
    let time_per_move1 = Duration::from_micros(2000);
    let time_per_move2 = Duration::from_micros(2000);

    let competition = Competition::new(
        Algorithm::new(modules1, time_per_move1),
        Algorithm::new(modules2, time_per_move2),
    );

    // competition.analyze_algorithm_choices(|(game_info, _), _| {
    //     game_info.outcome == GameOutcome::InconclusiveTooLong
    // });
    let results = competition.start_competition(5000).await;
    dbg!(results);
}
