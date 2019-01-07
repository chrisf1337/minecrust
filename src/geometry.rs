pub mod boundingbox;
pub mod ray;
pub mod rectangle;
pub mod square;
pub mod unitcube;

use crate::{geometry::boundingbox::BoundingBox, types::prelude::*, vulkan::Vertex3f};

pub trait PrimitiveGeometry: Send + Sync {
    /// Vertices to be copied to a vertex buffer.
    fn vtx_data(&self, transform: &Transform3f) -> Vec<Vertex3f>;

    fn vertices(&self, transform: &Transform3f) -> Vec<Point3f>;

    fn bounding_box(&self, transform: &Transform3f) -> BoundingBox;
}
