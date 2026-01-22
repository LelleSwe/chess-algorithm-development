use std::{collections::HashMap, hash::Hash};

use chess::Board;

use crate::algorithms::utils::Evaluation;

#[derive(Debug, Copy, Clone)]
pub struct TranspositionEntry {
    pub depth: u32,
    pub(crate) evaluation: Evaluation,
}

impl TranspositionEntry {
    pub(crate) fn new(depth: u32, evaluation: Evaluation) -> Self {
        TranspositionEntry { depth, evaluation }
    }
}

pub(crate) fn insert_in_transposition_table(
    transposition_table: &mut HashMap<u64, TranspositionEntry>,
    board: &Board,
    depth: u32,
    evaluation: Evaluation,
) {
    transposition_table.insert(board.get_hash(), TranspositionEntry::new(depth, evaluation));
}

pub(crate) fn get_transposition_entry(
    transposition_table: &HashMap<u64, TranspositionEntry>,
    board: &Board,
) -> Option<TranspositionEntry> {
    let transposition_entry = transposition_table.get(&board.get_hash()).copied();

    transposition_entry
}

#[derive(Default, Debug, Clone)]
pub struct TTEntry {
    pub depth: u32,
    pub(crate) evaluation: Evaluation,
    pub hash: u64,
}

impl PartialEq for TTEntry {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

const TTSIZE: usize = 1 << 19;
#[derive(Debug)]
pub struct TranspositionTable {
    data: Vec<TTEntry>,
}

impl TranspositionTable {
    pub fn new() -> Self {
        let mut vec = Vec::with_capacity(TTSIZE);
        vec.resize_with(TTSIZE, || TTEntry::default());
        Self { data: vec }
    }

    pub fn get(&self, hash: u64) -> Option<&TTEntry> {
        let mut idx: usize = hash as usize;
        idx = idx % TTSIZE;
        let entry = &self.data[idx];
        if entry.hash == hash {
            Some(entry)
        } else {
            None
        }
    }

    pub fn insert(&mut self, entry: TTEntry) {
        let entry2 = self.get(entry.hash.clone());
        match entry2 {
            None => {
                let mut idx: usize = entry.hash as usize;
                idx = idx % TTSIZE;
                self.data[idx] = entry;
            }
            Some(s) => {
                if s.depth <= entry.depth {
                    let mut idx: usize = entry.hash as usize;
                    idx = idx % TTSIZE;
                    self.data[idx] = entry;
                }
            }
        }
    }
}
