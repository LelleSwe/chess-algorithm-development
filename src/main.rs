use crate::algorithms::the_algorithm::Algorithm;
use crate::common::constants::modules::{ALPHA_BETA, ANALYZE};

use self::pitter::logic::Competition;

mod algorithms;
mod common;
mod pitter;

fn main() {
    let modules1 = ALPHA_BETA | ANALYZE;
    let modules2 = ANALYZE;

    let mut competition = Competition::new(Algorithm::new(modules1), Algorithm::new(modules2));

    // let results = competition.analyze_algorithm_choices(|_, _| true);
    let results = competition.start_competition();
    dbg!(results);
}
