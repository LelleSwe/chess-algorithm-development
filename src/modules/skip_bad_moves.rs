pub fn should_skip(num_legal_moves: usize, i: usize) -> bool {
    i as f32 > num_legal_moves as f32 * 1.
}
