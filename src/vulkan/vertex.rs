use crate::{types::prelude::*, vulkan::vertex_input::VertexInput};
use ash::vk;

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
}

impl VertexInput for Vertex2f {
    fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
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

    pub fn transform(&self, transform: &Transform3f) -> Vertex3f {
        Vertex3f::new(transform * self.pos, self.uv)
    }
}

impl VertexInput for Vertex3f {
    fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
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
}
