use std::{fs::OpenOptions, io::prelude::*};

use crate::common::constants::NUMBER_OF_MODULES;

pub(crate) fn write_result(buf: &[u8], file: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .unwrap();
    file.write_all(buf)?;
    Ok(())
}

pub(crate) fn modules_to_string(modules: u32) -> String {
    let mut start: bool = true;
    let mut output: String = "".to_string();
    //For loop ranges is all available modules index. +1 because we need inclusive.
    for i in 0..NUMBER_OF_MODULES + 1 {
        if (modules & (1 << i)).count_ones() == 1 {
            let module_string = match i {
                0 => "ANALYZE",
                1 => "ALPHA_BETA",
                2 => "TRANSPOSITION_TABLE",
                3 => "SEARCH_EXTENSIONS",
                4 => "SQUARE_CONTROL_METRIC",
                5 => "SKIP_BAD_MOVES",
                6 => "NAIVE_PSQT",
                7 => "PAWN_STRUCTURE",
                8 => "TAPERED_EVERY_PESTO_PSQT",
                9 => "TAPERED_INCREMENTAL_PESTO_PSQT",
                _ => "INVALID MODULE DETECTED",
            };
            if !start {
                output = output + ", " + module_string;
            } else {
                start = false;
                output = module_string.to_string();
            }
        }
    }
    output
}
