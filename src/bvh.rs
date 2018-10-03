fn median<T: PartialOrd + Clone>(v: &mut [T]) -> T {
    if v.is_empty() {
        panic!("Cannot take median of empty slice");
    }
    let len = v.len();
    v.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    v[len / 2].clone()
}
