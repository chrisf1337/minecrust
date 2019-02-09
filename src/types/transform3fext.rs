use crate::{na::Translation3, types::prelude::*};
use alga::general::SubsetOf;

pub trait Transform3fExt {
    fn new_with_translation(translation: Vector3f) -> Self;

    fn translation(&self) -> Point3f {
        Point3f::from(Vector3f::from_vector4f(self.translation_homogeneous()))
    }

    fn translation_homogeneous(&self) -> Vector4f;
}

impl Transform3fExt for Transform3f {
    fn new_with_translation(translation: Vector3f) -> Transform3f {
        Translation3::from(translation).to_superset()
    }

    fn translation_homogeneous(&self) -> Vector4f {
        self.matrix().translation_homogeneous()
    }
}
