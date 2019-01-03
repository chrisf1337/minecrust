mod color;
pub mod prelude;

pub use crate::types::color::Color;

use crate::na::{
    Isometry3, Matrix3, Matrix4, Point2, Point3, Quaternion, Rotation2, Rotation3, Transform2,
    Transform3, Translation2, UnitQuaternion, Vector2, Vector3, Vector4,
};

pub type Matrix3f = Matrix3<f32>;
pub type Matrix4f = Matrix4<f32>;
pub type Vector2f = Vector2<f32>;
pub type Vector3f = Vector3<f32>;
pub type Vector4f = Vector4<f32>;
pub type Point2f = Point2<f32>;
pub type Point3f = Point3<f32>;
pub type Quaternionf = Quaternion<f32>;
pub type UnitQuaternionf = UnitQuaternion<f32>;
pub type Translation2f = Translation2<f32>;
pub type Transform2f = Transform2<f32>;
pub type Transform3f = Transform3<f32>;
pub type Rotation3f = Rotation3<f32>;
pub type Isometry3f = Isometry3<f32>;

pub struct Vecf(pub Vec<f32>);

impl Vecf {
    pub fn from_slice(slice: &[f32]) -> Vecf {
        Vecf(slice.to_vec())
    }
}

impl PartialEq for Vecf {
    fn eq(&self, other: &Vecf) -> bool {
        if self.0.len() != other.0.len() {
            return false;
        }
        self.0.eq(&other.0)
    }
}
