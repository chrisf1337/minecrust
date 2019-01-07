use crate::{types::prelude::*, utils::f32};
use std::f32::consts::PI;

/// Returns the difference in yaw and pitch between `v1` and `v2`.
pub fn yaw_pitch_diff(v1: &Vector3f, v2: &Vector3f) -> (f32, f32) {
    let v1 = v1.normalize();
    let v2 = v2.normalize();
    let v1y = f32::atan2(v1.x, v1.z);
    let v1p = f32::asin(-v1.y);
    let v2y = f32::atan2(v2.x, v2.z);
    let v2p = f32::asin(-v2.y);
    let mut dy = v2y - v1y;
    let dp = v2p - v1p;
    if dy > PI {
        dy -= 2.0 * PI;
    } else if dy < -PI {
        dy += 2.0 * PI;
    }
    (dy, dp)
}
