use crate::types::prelude::*;

pub trait Vector4fExt {
    fn from_vector3f(v: Vector3f) -> Vector4f;
}

impl Vector4fExt for Vector4f {
    fn from_vector3f(v: Vector3f) -> Vector4f {
        Vector4f::new(v.x, v.y, v.z, 0.0)
    }
}
