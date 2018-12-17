use crate::na::{
    Matrix4, Point2, Point3, Quaternion, Rotation3, Transform3, UnitQuaternion, Vector3, Vector4,
};
use ash::vk;
use gl::types::GLenum;

pub type Matrix4f = Matrix4<f32>;
pub type Vector3f = Vector3<f32>;
pub type Vector4f = Vector4<f32>;
pub type Point2f = Point2<f32>;
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

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Vertex2f {
    pub pos: Point2f,
    pub uv: Point2f,
}

impl Vertex2f {
    pub fn new(pos: Point2f, uv: Point2f) -> Vertex2f {
        Vertex2f { pos, uv }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex2f, pos) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Vertex2f, uv) as u32)
                .build(),
        ]
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Vertex3f {
    pub pos: Point3f,
    pub uv: Point2f,
}

impl Vertex3f {
    pub fn new(pos: Point3f, uv: Point2f) -> Vertex3f {
        Vertex3f { pos, uv }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex3f, pos) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Vertex3f, uv) as u32)
                .build(),
        ]
    }

    pub fn transform(&self, transform: &Transform3f) -> Vertex3f {
        Vertex3f::new(transform * self.pos, self.uv)
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
