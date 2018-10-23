use crate::types::*;
use crate::utils::vec4f;

pub fn col_to_vec4f(m: &Matrix4f, c: usize) -> Vector4f {
    let col = m.column(c);
    Vector4f::new(col[0], col[1], col[2], col[3])
}

pub fn almost_eq(m1: &Matrix4f, m2: &Matrix4f) -> bool {
    vec4f::almost_eq(&col_to_vec4f(m1, 0), &col_to_vec4f(m2, 0))
        && vec4f::almost_eq(&col_to_vec4f(m1, 1), &col_to_vec4f(m2, 1))
        && vec4f::almost_eq(&col_to_vec4f(m1, 2), &col_to_vec4f(m2, 2))
        && vec4f::almost_eq(&col_to_vec4f(m1, 3), &col_to_vec4f(m2, 3))
}
