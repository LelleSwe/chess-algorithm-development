use crate::algorithms::the_algorithm::Algorithm;
use crate::common::constants::modules::ALPHA_BETA;

use self::pitter::logic::Competition;

mod algorithms;
mod common;
mod pitter;

fn main() {
    let modules1 = ALPHA_BETA;
    let modules2 = 0;

    let mut competition = Competition::new(Algorithm::new(modules1), Algorithm::new(modules2));

    let results = competition.start_competition();
    dbg!(results);
}
