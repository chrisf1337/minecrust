use crate::na::{Isometry, Rotation3, Translation3};
use crate::{
    geometry::{square::Square, BoundingBox, PrimitiveGeometry},
    types::prelude::*,
    vulkan::Vertex3f,
};
use alga::general::SubsetOf;
use std::f32::consts::{FRAC_PI_2, PI};

#[derive(Clone, Debug)]
pub struct UnitCube {
    pub side_len: f32,
    transforms: Vec<Transform3f>,
}

impl UnitCube {
    pub fn new(side_len: f32) -> UnitCube {
        let transforms = vec![
            // top
            Translation3::from(Vector3f::new(0.0, side_len / 2.0, 0.0)).to_superset(),
            Isometry::from_parts(
                Translation3::from(Vector3f::new(0.0, 0.0, side_len / 2.0)),
                Rotation3::from_axis_angle(&Vector3f::x_axis(), FRAC_PI_2),
            )
            .to_superset(),
            Isometry::from_parts(
                Translation3::from(Vector3f::new(side_len / 2.0, 0.0, 0.0)),
                Rotation3::from_axis_angle(&Vector3f::z_axis(), -FRAC_PI_2),
            )
            .to_superset(),
            Isometry::from_parts(
                Translation3::from(Vector3f::new(0.0, 0.0, -side_len / 2.0)),
                Rotation3::from_axis_angle(&Vector3f::x_axis(), -FRAC_PI_2),
            )
            .to_superset(),
            Isometry::from_parts(
                Translation3::from(Vector3f::new(-side_len / 2.0, 0.0, 0.0)),
                Rotation3::from_axis_angle(&Vector3f::z_axis(), FRAC_PI_2),
            )
            .to_superset(),
            Isometry::from_parts(
                Translation3::from(Vector3f::new(0.0, -side_len / 2.0, 0.0)),
                Rotation3::from_axis_angle(&Vector3f::x_axis(), PI),
            )
            .to_superset(),
        ];
        UnitCube {
            side_len,
            transforms,
        }
    }
}

impl PrimitiveGeometry for UnitCube {
    fn vtx_data(&mut self, transform: &Transform3f) -> Vec<Vertex3f> {
        let mut sq = Square::new(self.side_len);
        let mut vertices = vec![];
        for tr in &self.transforms {
            vertices.extend_from_slice(&sq.vtx_data(&(transform * tr)));
        }
        vertices
    }

    fn vertices(&mut self, transform: &Transform3f) -> Vec<Point3f> {
        let mut sq = Square::new(self.side_len);
        let mut vertices = vec![];
        for tr in &self.transforms {
            vertices.extend(sq.vertices(&(transform * tr)));
        }
        vertices
    }

    fn bounding_box(&self, transform: &Transform3f) -> BoundingBox {
        let min = transform * Point3f::new(-self.side_len, -self.side_len, -self.side_len);
        let max = transform * Point3f::new(self.side_len, self.side_len, self.side_len);
        BoundingBox::new(min, max)
    }
}
