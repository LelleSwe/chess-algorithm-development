#[allow(unused_imports)]
use crate::common::constants::{
    modules::{
        ALPHA_BETA, ANALYZE, NAIVE_PSQT, PAWN_STRUCTURE, SEARCH_EXTENSIONS, SKIP_BAD_MOVES,
        SQUARE_CONTROL_METRIC, TAPERED_EVERY_PESTO_PSQT, TAPERED_INCREMENTAL_PESTO_PSQT,
        TRANSPOSITION_TABLE,
    },
    NUMBER_OF_MODULES,
};
use crate::{algorithms::the_algorithm::Algorithm, timestat::TimeStat};
use chess::Board;
use chess::ChessMove;

use std::{io::*, str::FromStr};

mod algorithms;
mod common;
mod io;
mod modules;
mod timestat;
mod tt;

fn main() {
    let mut board = Board::default();
    let mut algo = Algorithm::new(TimeStat::new());

    let inp = stdin();
    loop {
        let mut buf = String::new();
        buf.clear();
        let _ = inp.read_line(&mut buf);
        let mut parts = buf.split_ascii_whitespace();
        let arg1 = match parts.next() {
            None => "",
            Some(s) => s,
        };

        match arg1 {
            "uci" => {
                println!("id name chess-bot");
                println!("id author Mostafa Kerim");
                println!("id author Leonard Jarlskog");
                println!("uciok");
            }

            "position" => {
                while let Some(arg) = parts.next() {
                    let _ = match arg {
                        "fen" => {
                            let fen = format!(
                                "{} {} {} {} {} {}",
                                parts.next().unwrap(),
                                parts.next().unwrap(),
                                parts.next().unwrap(),
                                parts.next().unwrap(),
                                parts.next().unwrap(),
                                parts.next().unwrap()
                            );
                            board = Board::from_str(&fen).unwrap();
                        }
                        "startpos" => {
                            board = Board::default();
                        }
                        "moves" => {
                            while let Some(movestr) = parts.next() {
                                let mov = ChessMove::from_str(movestr).unwrap();
                                board = board.make_move_new(mov);
                            }
                        }
                        _ => (),
                    };
                }
            }

            "go" => {
                while let Some(arg) = parts.next() {
                    let _ = match arg {
                        "btime" => algo.timestat.add_btime(parts.next().unwrap()),
                        "wtime" => algo.timestat.add_wtime(parts.next().unwrap()),
                        "binc" => algo.timestat.add_binc(parts.next().unwrap()),
                        "winc" => algo.timestat.add_winc(parts.next().unwrap()),
                        "movestogo" => algo.timestat.add_movestogo(parts.next().unwrap()),
                        "depth" => algo.timestat.add_depth(parts.next().unwrap()),
                        "movetime" => algo.timestat.add_movetime(parts.next().unwrap()),
                        _ => Ok(()),
                    };
                }
                let best_move = algo.next_action_iterative_deepening(&board);
                println!("bestmove {}", best_move);
            }

            "playself" => {
                let mut nps: Vec<i64> = Vec::new();
                loop {
                    let _ = algo.timestat.add_movetime("1000");
                    let best_move = algo.next_action_iterative_deepening(&board);
                    println!("{}", best_move);
                    nps.push(
                        algo.timestat.nps_vec.iter().map(|e| *e).sum::<i64>()
                            / algo.timestat.nps_vec.len() as i64,
                    );
                    if best_move == ChessMove::default() {
                        break;
                    }
                    board = board.make_move_new(best_move);
                    println!("{}", nps.iter().map(|e| *e).sum::<i64>() / nps.len() as i64);
                }
            }

            "isready" => {
                println!("readyok");
            }

            "quit" => {
                break;
            }

            _ => (),
        }

        let _ = stdout().flush();
    }
}
