use crate::types::prelude::*;

pub fn from_vec3f(v: Vector3f) -> Vector4f {
    Vector4f::new(v.x, v.y, v.z, 0.0)
}
