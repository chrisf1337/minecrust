use crate::types::prelude::*;
use crate::utils::{mat3f, vec4f};
use num_traits::identities::Zero;

pub fn from_mat3f(m: Matrix3f) -> Matrix4f {
    Matrix4f::from_columns(&[
        vec4f::from_vec3f(mat3f::col_to_vec3f(&m, 0)),
        vec4f::from_vec3f(mat3f::col_to_vec3f(&m, 1)),
        vec4f::from_vec3f(mat3f::col_to_vec3f(&m, 2)),
        Vector4f::zero(),
    ])
}
