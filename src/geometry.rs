pub mod boundingbox;
pub mod ray;
pub mod rectangle;
pub mod square;
pub mod unitcube;

use crate::geometry::boundingbox::BoundingBox;
use crate::types::*;

pub trait PrimitiveGeometry {
    fn vtx_data(&mut self, transform: &Transform3f) -> Vec<f32>;
    fn vertices(&mut self, transform: &Transform3f) -> Vec<Point3f>;
    fn bounding_box(&self, transform: &Transform3f) -> BoundingBox;
}
