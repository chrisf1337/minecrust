const FLOAT_COMPARISON_EPSILON: f32 = 1.0e-6;

pub fn almost_eq(a: f32, b: f32) -> bool {
    f32::abs(a - b) <= FLOAT_COMPARISON_EPSILON
}
