use crate::na::{Isometry, Rotation3, Translation3};
use crate::{
    geometry::{square::Square, Aabb, PrimitiveGeometry},
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
            // front
            Isometry::from_parts(
                Translation3::from(Vector3f::new(0.0, 0.0, side_len / 2.0)),
                Rotation3::from_axis_angle(&Vector3f::x_axis(), FRAC_PI_2),
            )
            .to_superset(),
            // left
            Isometry::from_parts(
                Translation3::from(Vector3f::new(-side_len / 2.0, 0.0, 0.0)),
                Rotation3::from_axis_angle(&Vector3f::z_axis(), FRAC_PI_2),
            )
            .to_superset(),
            // right
            Isometry::from_parts(
                Translation3::from(Vector3f::new(side_len / 2.0, 0.0, 0.0)),
                Rotation3::from_axis_angle(&Vector3f::z_axis(), -FRAC_PI_2),
            )
            .to_superset(),
            // back
            Isometry::from_parts(
                Translation3::from(Vector3f::new(0.0, 0.0, -side_len / 2.0)),
                Rotation3::from_axis_angle(&Vector3f::x_axis(), -FRAC_PI_2),
            )
            .to_superset(),
            // bottom
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
    fn vtx_data(&self, transform: &Transform3f) -> Vec<Vertex3f> {
        let sq = Square::new(self.side_len);
        let mut vertices = vec![];
        for tr in &self.transforms {
            vertices.extend(sq.vtx_data(&(transform * tr)));
        }
        vertices
    }

    fn vtx_pts(&self, transform: &Transform3f) -> Vec<Point3f> {
        let sq = Square::new(self.side_len);
        let mut vertices = vec![];
        for tr in &self.transforms {
            vertices.extend(sq.vtx_pts(&(transform * tr)));
        }
        vertices
    }

    fn aabb(&self, transform: &Transform3f) -> Aabb {
        Aabb::new(
            transform.translation(),
            Vector3f::new(
                self.side_len / 2.0,
                self.side_len / 2.0,
                self.side_len / 2.0,
            ),
        )
    }
}
