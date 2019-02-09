use crate::types::prelude::*;
use num_traits::identities::Zero;

pub trait Matrix4fExt: Transform3fExt {
    fn from_matrix3f(m: Matrix3f) -> Matrix4f;
}

impl Transform3fExt for Matrix4f {
    fn new_with_translation(translation: Vector3f) -> Matrix4f {
        Transform3f::new_with_translation(translation).to_homogeneous()
    }

    fn translation_homogeneous(&self) -> Vector4f {
        let col = self.column(3);
        Vector4f::new(col[0], col[1], col[2], col[3])
    }
}

impl Matrix4fExt for Matrix4f {
    fn from_matrix3f(m: Matrix3f) -> Matrix4f {
        Matrix4f::from_columns(&[
            Vector4f::from_vector3f(Matrix3f::column_to_vector3f(&m, 0)),
            Vector4f::from_vector3f(Matrix3f::column_to_vector3f(&m, 1)),
            Vector4f::from_vector3f(Matrix3f::column_to_vector3f(&m, 2)),
            Vector4f::zero(),
        ])
    }
}
