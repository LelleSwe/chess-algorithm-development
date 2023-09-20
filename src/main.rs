use crate::algorithms::random_move::InstaResign;

use self::algorithms::random_move::RandomMove;
use self::pitter::logic::Competition;

mod algorithms;
mod common;
mod pitter;

fn main() {
    let algo1 = RandomMove;
    let algo2 = InstaResign;

    let mut competition = Competition::new(Box::new(algo1), Box::new(algo2));
    let results = competition.start_competition();
    dbg!(results);
}
