use crate::types::*;

pub fn from_vec4f(v: &Vector4f) -> Vector3f {
    Vector3f::new(v.x, v.y, v.z)
}
