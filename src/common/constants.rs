pub(crate) mod modules {
    pub(crate) const ANALYZE: u32 = 0b1;
    pub(crate) const ALPHA_BETA: u32 = 0b10;
    pub(crate) const TRANSPOSITION_TABLE: u32 = 0b100;
    pub(crate) const SEARCH_EXTENSIONS: u32 = 0b1000;
    pub(crate) const SQUARE_CONTROL_METRIC: u32 = 0b10000;
    pub(crate) const SKIP_BAD_MOVES: u32 = 0b100000;
}
