mod color;
pub mod prelude;

pub use crate::types::color::Color;

use crate::na::{
    Matrix3, Matrix4, Point2, Point3, Transform2, Transform3, UnitQuaternion, Vector2, Vector3,
    Vector4,
};

pub type Matrix3f = Matrix3<f32>;
pub type Matrix4f = Matrix4<f32>;
pub type Vector2f = Vector2<f32>;
pub type Vector3f = Vector3<f32>;
pub type Vector4f = Vector4<f32>;
pub type Point2f = Point2<f32>;
pub type Point3f = Point3<f32>;
pub type UnitQuaternionf = UnitQuaternion<f32>;
pub type Transform2f = Transform2<f32>;
pub type Transform3f = Transform3<f32>;
