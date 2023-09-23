use crate::algorithms::basic_no_stalemate::BasicNoStalemateAlgo;

use self::algorithms::random_move::RandomMove;
use self::pitter::logic::Competition;

mod algorithms;
mod common;
mod pitter;

fn main() {
    type Algo1 = RandomMove;
    type Algo2 = BasicNoStalemateAlgo;

    let mut competition = Competition::new(Box::new(Algo1 {}), Box::new(Algo2 {}));

    let results = competition.start_competition();
    dbg!(results);
}
