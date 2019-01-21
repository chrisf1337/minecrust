use crate::{na::Translation3, types::prelude::*};
use alga::general::SubsetOf;

pub trait Transform3fExt {
    fn new_with_translation(translation: Vector3f) -> Transform3f;
}

impl Transform3fExt for Transform3f {
    fn new_with_translation(translation: Vector3f) -> Transform3f {
        Translation3::from(translation).to_superset()
    }
}
