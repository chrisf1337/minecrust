pub mod f32;
pub mod mat4f;
pub mod pt3f;
pub mod quat4f;
pub mod vec3f;
pub mod vec4f;

pub const NSEC_PER_SEC: u32 = 1_000_000_000;

pub fn min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b {
        a
    } else {
        b
    }
}

pub fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b {
        a
    } else {
        b
    }
}

pub fn clamp<T: PartialOrd>(min: T, max: T, t: T) -> T {
    assert!(max >= min);
    self::min(min, self::max(max, t))
}
