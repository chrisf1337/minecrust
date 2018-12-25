mod color;
mod text;
mod vertex;

pub use crate::types::{
    color::Color,
    text::TextVertex,
    vertex::{Vertex2f, Vertex3f},
};

use crate::na::{
    Isometry3, Matrix4, Point2, Point3, Quaternion, Rotation3, Transform3, UnitQuaternion, Vector3,
    Vector4,
};
use ash::vk;

pub type Matrix4f = Matrix4<f32>;
pub type Vector3f = Vector3<f32>;
pub type Vector4f = Vector4<f32>;
pub type Point2f = Point2<f32>;
pub type Point3f = Point3<f32>;
pub type Quaternionf = Quaternion<f32>;
pub type UnitQuaternionf = UnitQuaternion<f32>;
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
