use crate::algorithms::utils::Evaluation;

pub(crate) fn calc_new(
    mut alpha: f32,
    mut beta: f32,
    maximise: bool,
    evaluation: Evaluation,
) -> (f32, f32) {
    if let Some(eval) = evaluation.eval {
        if maximise {
            alpha = alpha.max(eval);
        } else {
            beta = beta.min(eval);
        }
    }
    return (alpha, beta);
}
