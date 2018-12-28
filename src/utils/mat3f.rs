use crate::types::prelude::*;

pub fn col_to_vec3f(m: &Matrix3f, c: usize) -> Vector3f {
    let col = m.column(c);
    Vector3f::new(col[0], col[1], col[2])
}
