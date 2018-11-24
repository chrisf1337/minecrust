use crate::na::{
    Matrix4, Point3, Quaternion, Rotation3, Transform3, UnitQuaternion, Vector3, Vector4,
};
use gl::types::GLenum;

pub type Matrix4f = Matrix4<f32>;
pub type Vector3f = Vector3<f32>;
pub type Vector4f = Vector4<f32>;
pub type Point3f = Point3<f32>;
pub type Quaternionf = Quaternion<f32>;
pub type UnitQuaternionf = UnitQuaternion<f32>;
pub type Transform3f = Transform3<f32>;
pub type Rotation3f = Rotation3<f32>;

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

pub enum GlBufferUsage {
    StreamDraw,
    StreamRead,
    StreamCopy,
    StaticDraw,
    StaticRead,
    StaticCopy,
    DynamicDraw,
    DynamicRead,
    DynamicCopy,
}

impl Into<GLenum> for GlBufferUsage {
    fn into(self) -> GLenum {
        match self {
            GlBufferUsage::StreamDraw => gl::STREAM_DRAW,
            GlBufferUsage::StreamRead => gl::STREAM_READ,
            GlBufferUsage::StreamCopy => gl::STREAM_COPY,
            GlBufferUsage::StaticDraw => gl::STATIC_DRAW,
            GlBufferUsage::StaticRead => gl::STATIC_READ,
            GlBufferUsage::StaticCopy => gl::STATIC_COPY,
            GlBufferUsage::DynamicDraw => gl::DYNAMIC_DRAW,
            GlBufferUsage::DynamicRead => gl::DYNAMIC_READ,
            GlBufferUsage::DynamicCopy => gl::DYNAMIC_COPY,
        }
    }
}
