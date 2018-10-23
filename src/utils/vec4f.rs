use crate::types::*;
use crate::utils::f32;

pub fn almost_eq(v1: &Vector4f, v2: &Vector4f) -> bool {
    f32::almost_eq(v1.x, v2.x)
        && f32::almost_eq(v1.y, v2.y)
        && f32::almost_eq(v1.z, v2.z)
        && f32::almost_eq(v1.w, v2.w)
}
