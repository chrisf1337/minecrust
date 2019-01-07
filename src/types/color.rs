use crate::types::prelude::*;

/// Color from 0.0 to 1.0.
#[derive(Debug, Copy, Clone)]
pub struct Color(Vector4f);

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Color {
        Color(Vector4f::new(r, g, b, 1.0))
    }
}
