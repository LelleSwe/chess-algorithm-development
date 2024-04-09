use std::fs::remove_file;
use std::mem;
use std::time::Duration;

use crate::algorithms::the_algorithm::Algorithm;
#[allow(unused_imports)]
use crate::common::constants::{
    modules::{
        ALPHA_BETA, ANALYZE, NAIVE_PSQT, PAWN_STRUCTURE, SEARCH_EXTENSIONS, SKIP_BAD_MOVES,
        SQUARE_CONTROL_METRIC, TAPERED_EVERY_PESTO_PSQT, TAPERED_INCREMENTAL_PESTO_PSQT,
        TRANSPOSITION_TABLE,
    },
    NUMBER_OF_MODULES,
};
use crate::io::write_result;

use self::pitter::logic::{Competition, CompetitionResults};

mod algorithms;
mod common;
mod io;
mod modules;
mod pitter;

//If we should test all possible pairs of combinations.
const TEST_ALL_PAIRS: bool = false;

#[tokio::main]
async fn main() {
    remove_file("./output.txt").unwrap_or_default();
    if !TEST_ALL_PAIRS {
        //ALPHA_BETA | ANALYZE | SEARCH_EXTENSIONS | SKIP_BAD_MOVES | SQUARE_CONTROL_METRIC | TRANSPOSITION_TABLE | NAIVE_PSQT | PAWN_STRUCTURE | TAPERED_EVERY_PESTO_PSQT | TAPERED_INCREMENTAL_PESTO_PSQT
        //Put 0 for no modules.
        //Setup modules
        let modules1 = ALPHA_BETA | TAPERED_EVERY_PESTO_PSQT;
        let modules2 = ALPHA_BETA;
        let time_per_move1 = Duration::from_micros(2000);
        let time_per_move2 = Duration::from_micros(2000);
        let game_pairs = 1000;

        //Run competition
        let result = do_competition(
            modules1,
            modules2,
            time_per_move1,
            time_per_move2,
            game_pairs,
        )
        .await;
    
        println!("Algo 1: {}", io::modules_to_string(modules1));
        println!("Algo 2: {}", io::modules_to_string(modules2));
        println!("Game pairs: {}", game_pairs);
        dbg!(result);
    } else {
        println!(
            "Running {} possibilites",
            (NUMBER_OF_MODULES + 1) * NUMBER_OF_MODULES / 2
        );
        let time_per_move1 = Duration::from_micros(2000);
        let time_per_move2 = Duration::from_micros(2000);
        let game_pairs = 400;

        let mut competitions_run: u32 = 0;
        let mut dp: Vec<Vec<Option<CompetitionResults>>> =
            vec![vec![None; NUMBER_OF_MODULES]; NUMBER_OF_MODULES];
        for i in 0..NUMBER_OF_MODULES {
            for j in 0..NUMBER_OF_MODULES {
                if dp[j][i].is_some() {
                    let mut temp = dp[j][i].unwrap();
                    let output: String =
                        format!("{}\t", temp.algo2_wins as i64 - temp.algo1_wins as i64);
                    let buf = output.as_bytes();
                    let _ = write_result(buf, "./output.txt");
                    mem::swap(&mut temp.algo1_wins, &mut temp.algo2_wins);
                    dp[i][j] = Some(temp);
                    continue;
                }
                competitions_run += 1;
                println!(
                    "\rTesting pair {} out of {}",
                    competitions_run,
                    (NUMBER_OF_MODULES + 1) * NUMBER_OF_MODULES / 2
                );

                // Analyze is useless in this scenario
                let modules1 = 1 << j;
                let modules1 = if modules1 == ANALYZE { 0 } else { modules1 };
                let modules2 = 1 << i;
                let modules2 = if modules2 == ANALYZE { 0 } else { modules2 };

                let result = do_competition(
                    modules1,
                    modules2,
                    time_per_move1,
                    time_per_move2,
                    game_pairs,
                )
                .await;

                dp[i][j] = Some(result);
                let output: String =
                    format!("{}\t", result.algo1_wins as i64 - result.algo2_wins as i64);
                let buf = output.as_bytes();
                let _ = write_result(buf, "./output.txt");
            }
            let _ = write_result("\n".as_bytes(), "./output.txt");
        }
    }
}

async fn do_competition(
    modules1: u32,
    modules2: u32,
    time_per_move1: Duration,
    time_per_move2: Duration,
    game_pairs: u32,
) -> CompetitionResults {
    let competition = Competition::new(
        Algorithm::new(modules1, time_per_move1),
        Algorithm::new(modules2, time_per_move2),
    );

    // competition.analyze_algorithm_choices(|(game_info, _), _| {
    //     game_info.outcome == GameOutcome::InconclusiveTooLong
    // });
    competition.start_competition(game_pairs).await
}
