use crate::types::*;

/// Color from 0.0 to 1.0.
#[derive(Debug, Copy, Clone)]
pub struct Color(Vector4f);

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Color {
        Color(Vector4f::new(r, g, b, 1.0))
    }

    pub fn new_with_alpha(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color(Vector4f::new(r, g, b, a))
    }

    pub fn r(&self) -> f32 {
        self.0.x
    }

    pub fn g(&self) -> f32 {
        self.0.y
    }

    pub fn b(&self) -> f32 {
        self.0.z
    }

    pub fn a(&self) -> f32 {
        self.0.w
    }

    pub fn as_slice(&self) -> &[f32] {
        self.0.as_slice()
    }
}
