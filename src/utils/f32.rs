pub fn mean(fs: &[f32]) -> f32 {
    fs.iter().sum::<f32>() / fs.len() as f32
}

pub fn max_many(fs: &[f32]) -> f32 {
    assert!(!fs.is_empty());
    *fs.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap()
}

pub fn max_index(fs: &[f32]) -> usize {
    assert!(!fs.is_empty());
    fs.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap()
        .0
}

pub fn min_many(fs: &[f32]) -> f32 {
    assert!(!fs.is_empty());
    *fs.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap()
}

pub fn min_index(fs: &[f32]) -> usize {
    assert!(!fs.is_empty());
    fs.iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap()
        .0
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
