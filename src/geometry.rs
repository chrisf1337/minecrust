mod aabb;
mod aap;
mod ray;
mod rectangle;
mod square;
mod unitcube;

pub use self::{aabb::AABB, ray::Ray, rectangle::Rectangle, square::Square, unitcube::UnitCube};

use crate::{types::prelude::*, vulkan::Vertex3f};

pub trait PrimitiveGeometry: Send + Sync {
    /// Vertices to be copied to a vertex buffer.
    fn vtx_data(&self, transform: &Transform3f) -> Vec<Vertex3f>;

    fn vertices(&self, transform: &Transform3f) -> Vec<Point3f>;

    fn bounding_box(&self, transform: &Transform3f) -> AABB;
}
