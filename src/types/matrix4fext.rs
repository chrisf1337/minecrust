use crate::{types::prelude::*, utils::vec3f::Vector3fExt};

pub trait Matrix4fExt {
    fn translation(&self) -> Point3f {
        Point3f::from(Vector3f::from_vec4f(self.translation_homogeneous()))
    }

    fn translation_homogeneous(&self) -> Vector4f;
}

impl Matrix4fExt for Transform3f {
    fn translation_homogeneous(&self) -> Vector4f {
        self.matrix().translation_homogeneous()
    }
}

impl Matrix4fExt for Matrix4f {
    fn translation_homogeneous(&self) -> Vector4f {
        let col = self.column(3);
        Vector4f::new(col[0], col[1], col[2], col[3])
    }
}
