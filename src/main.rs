use std::time::Duration;


use crate::algorithms::the_algorithm::Algorithm;
#[allow(unused_imports)]
use crate::common::constants::modules::{
    ALPHA_BETA, ANALYZE, NAIVE_PSQT, PAWN_STRUCTURE, SEARCH_EXTENSIONS, SKIP_BAD_MOVES,
    SQUARE_CONTROL_METRIC, TAPERED_EVERY_PESTO_PSQT, TAPERED_INCREMENTAL_PESTO_PSQT,
    TRANSPOSITION_TABLE,
};
use crate::io::{write_result, modules_to_string};

use self::pitter::logic::{Competition, CompetitionResults};

mod algorithms;
mod common;
mod modules;
mod pitter;
mod io;

//If we should print the moves played and results of each game.
pub(crate) const PRINT_GAME: bool = false;

#[tokio::main]
async fn main() {
    if !TEST_SEVERAL {
        //ALPHA_BETA | ANALYZE | SEARCH_EXTENSIONS | SKIP_BAD_MOVES | SQUARE_CONTROL_METRIC | TRANSPOSITION_TABLE | NAIVE_PSQT | PAWN_STRUCTURE | TAPERED_EVERY_PESTO_PSQT | TAPERED_INCREMENTAL_PESTO_PSQT
        //Put 0 for no modules.
        //Setup modules
        let modules1 = ALPHA_BETA | TAPERED_INCREMENTAL_PESTO_PSQT;
        let modules2 = ALPHA_BETA | TAPERED_EVERY_PESTO_PSQT;
        let time_per_move1 = Duration::from_micros(2000);
        let time_per_move2 = Duration::from_micros(2000);
        let game_pairs = 500;

        //Run competition
        let result = do_competition(modules1, modules2, time_per_move1, time_per_move2, game_pairs).await;
        
        //Save results to file
        let mut output: String = "Algo 1 modules: ".to_owned() + &modules_to_string(modules1) + "\nAlgo 2 modules: " + &modules_to_string(modules2);
        let result = format!("\nCompetition results: {:#?}", &result);
        output = output + &result;
        let buf = output.as_bytes();
        let _ = write_result(buf, "./output.txt");

    } else {
        /*let time_per_move1 = Duration::from_micros(2000);
        let time_per_move2 = Duration::from_micros(2000);
        
        let mut modules1 = 1;
        let mut modules2 = 1;

        for i in 0..log2() {
            for j in 0.. {

            }
        }*/
    }
    
}

const TEST_SEVERAL: bool = false;
async fn do_competition(modules1: u32, modules2: u32, time_per_move1: Duration, time_per_move2: Duration, game_pairs: u32) -> CompetitionResults {
    let competition = Competition::new(
        Algorithm::new(modules1, time_per_move1),
        Algorithm::new(modules2, time_per_move2),
    );

    // competition.analyze_algorithm_choices(|(game_info, _), _| {
    //     game_info.outcome == GameOutcome::InconclusiveTooLong
    // });
    let results = competition.start_competition(game_pairs).await;
    results
}
