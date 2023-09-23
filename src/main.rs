use crate::algorithms::v2_basic_no_stalemate::BasicNoStalemateAlgo;
use crate::algorithms::v3_variable_depth::VariableDepthAlgo;

use self::pitter::logic::Competition;

mod algorithms;
mod common;
mod pitter;

fn main() {
    type Algo1 = VariableDepthAlgo;
    type Algo2 = BasicNoStalemateAlgo;

    let mut competition = Competition::new(Box::new(Algo1 {}), Box::new(Algo2 {}));

    let results = competition.start_competition();
    dbg!(results);
}
