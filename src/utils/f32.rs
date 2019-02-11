pub fn mean(fs: &[f32]) -> f32 {
    fs.iter().sum::<f32>() / fs.len() as f32
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
