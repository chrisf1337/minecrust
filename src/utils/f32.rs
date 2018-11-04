use std::f32;
const FLOAT_COMPARISON_EPSILON: f32 = 1.0e-6;

pub fn almost_eq(a: f32, b: f32) -> bool {
    f32::abs(a - b) <= FLOAT_COMPARISON_EPSILON
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

pub fn clerp(a: f32, b: f32, t: f32) -> f32 {
    if t <= 0.0 {
        a
    } else if t >= 1.0 {
        b
    } else {
        self::lerp(a, b, t)
    }
}

pub fn clamp(min: f32, max: f32, t: f32) -> f32 {
    f32::min(f32::max(t, max), min)
}
