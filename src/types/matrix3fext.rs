use crate::types::prelude::*;

pub trait Matrix3fExt {
    fn column_to_vector3f(&self, c: usize) -> Vector3f;
}

impl Matrix3fExt for Matrix3f {
    fn column_to_vector3f(&self, c: usize) -> Vector3f {
        let col = self.column(c);
        Vector3f::new(col[0], col[1], col[2])
    }
}
