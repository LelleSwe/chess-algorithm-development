use std::time::Duration;

use crate::algorithms::the_algorithm::Algorithm;
use crate::common::constants::modules::{ALPHA_BETA, ANALYZE, TRANSPOSITION_TABLE};

use self::pitter::logic::Competition;

mod algorithms;
mod common;
mod pitter;

fn main() {
    let modules1 = TRANSPOSITION_TABLE | ALPHA_BETA | ANALYZE;
    let modules2 = ALPHA_BETA | ANALYZE;
    let time_per_move1 = Duration::from_micros(2000);
    let time_per_move2 = Duration::from_micros(2000);

    let mut competition = Competition::new(
        Algorithm::new(modules1, time_per_move1),
        Algorithm::new(modules2, time_per_move2),
    );

    // let results = competition.analyze_algorithm_choices(|_, _| true);
    let results = competition.start_competition(200);
    dbg!(results);
}
