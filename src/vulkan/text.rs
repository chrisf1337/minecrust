use crate::types::{prelude::*, Color};
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

#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    pub width: f32,
    pub height: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance: f32,
}

impl From<freetype::GlyphMetrics> for GlyphMetrics {
    fn from(metrics: freetype::GlyphMetrics) -> GlyphMetrics {
        GlyphMetrics {
            width: metrics.width as f32 / 64.0,
            height: metrics.height as f32 / 64.0,
            bearing_x: metrics.horiBearingX as f32 / 64.0,
            bearing_y: metrics.horiBearingY as f32 / 64.0,
            advance: metrics.horiAdvance as f32 / 64.0,
        }
    }
}
