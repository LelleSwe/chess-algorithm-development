use crate::algorithms::v0_random_move::RandomMove;
use crate::algorithms::v1_basic::BasicAlgo;
use crate::algorithms::v2_basic_no_stalemate::BasicNoStalemateAlgo;

use self::pitter::logic::Competition;

mod algorithms;
mod common;
mod pitter;

fn main() {
    type Algo1 = BasicAlgo;
    type Algo2 = BasicNoStalemateAlgo;

    let mut competition = Competition::new(Box::new(Algo1 {}), Box::new(Algo2 {}));

    let results = competition.start_competition();
    dbg!(results);
}
