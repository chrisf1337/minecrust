use crate::types::*;
use ash::vk;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TextVertex {
    char_idx: usize,
    pub vertex: Point2f,
    uv: Point2f,
    color: Color,
}

impl TextVertex {
    pub fn new(char_idx: usize, vertex: Point2f, uv: Point2f, color: Color) -> TextVertex {
        TextVertex {
            char_idx,
            vertex,
            uv,
            color,
        }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 4] {
        [
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32_UINT)
                .offset(offset_of!(TextVertex, char_idx) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(TextVertex, vertex) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(TextVertex, uv) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(3)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(TextVertex, color) as u32)
                .build(),
        ]
    }
}
