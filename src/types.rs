mod color;
mod f32ext;
mod matrix3fext;
mod matrix4fext;
mod point2fext;
mod point3fext;
pub mod prelude;
mod transform3fext;
mod vector2fext;
mod vector3fext;
mod vector4fext;

pub use crate::types::color::Color;

use crate::na::{
    Affine2, Affine3, Matrix3, Matrix4, Point2, Point3, Translation3, UnitQuaternion, Vector2,
    Vector3, Vector4,
};

pub type Matrix3f = Matrix3<f32>;
pub type Matrix4f = Matrix4<f32>;
pub type Vector2f = Vector2<f32>;
pub type Vector3f = Vector3<f32>;
pub type Vector4f = Vector4<f32>;
pub type Point2f = Point2<f32>;
pub type Point3f = Point3<f32>;
pub type UnitQuaternionf = UnitQuaternion<f32>;
pub type Transform2f = Affine2<f32>;
pub type Transform3f = Affine3<f32>;
pub type Translation3f = Translation3<f32>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Face {
    Top,
    Front,
    Left,
    Right,
    Back,
    Bottom,
}
