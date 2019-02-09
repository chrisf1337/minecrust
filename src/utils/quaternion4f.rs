use crate::types::prelude::*;

pub fn clerp(a: &UnitQuaternionf, b: &UnitQuaternionf, t: f32) -> UnitQuaternionf {
    if t <= 0.0 {
        *a
    } else if t >= 1.0 {
        *b
    } else {
        a.nlerp(b, t)
    }
}
