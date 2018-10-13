use alga::general::SubsetOf;
use crate::geometry::square::Square;
use crate::na::{Isometry, Rotation3, Translation3};
use crate::types::*;
use std::f32::consts::{FRAC_PI_2, PI};

pub struct UnitCube {
    pub side_len: f32,
    squares: Vec<Square>,
}

impl UnitCube {
    pub fn new(side_len: f32) -> UnitCube {
        let squares = vec![
            // top
            Square::new_with_transform(
                side_len,
                &Translation3::from_vector(Vector3f::new(0.0, side_len / 2.0, 0.0)).to_superset(),
            ),
            // front
            Square::new_with_transform(
                side_len,
                &Isometry::from_parts(
                    Translation3::from_vector(Vector3f::new(0.0, 0.0, side_len / 2.0)),
                    Rotation3::from_axis_angle(&Vector3f::x_axis(), FRAC_PI_2),
                )
                .to_superset(),
            ),
            // right
            Square::new_with_transform(
                side_len,
                &Isometry::from_parts(
                    Translation3::from_vector(Vector3f::new(side_len / 2.0, 0.0, 0.0)),
                    Rotation3::from_axis_angle(&Vector3f::z_axis(), -FRAC_PI_2),
                )
                .to_superset(),
            ),
            // back
            Square::new_with_transform(
                side_len,
                &Isometry::from_parts(
                    Translation3::from_vector(Vector3f::new(0.0, 0.0, -side_len / 2.0)),
                    Rotation3::from_axis_angle(&Vector3f::x_axis(), -FRAC_PI_2),
                )
                .to_superset(),
            ),
            // left
            Square::new_with_transform(
                side_len,
                &Isometry::from_parts(
                    Translation3::from_vector(Vector3f::new(-side_len / 2.0, 0.0, 0.0)),
                    Rotation3::from_axis_angle(&Vector3f::z_axis(), FRAC_PI_2),
                )
                .to_superset(),
            ),
            // bottom
            Square::new_with_transform(
                side_len,
                &Isometry::from_parts(
                    Translation3::from_vector(Vector3f::new(0.0, -side_len / 2.0, 0.0)),
                    Rotation3::from_axis_angle(&Vector3f::x_axis(), PI),
                )
                .to_superset(),
            ),
        ];
        UnitCube { side_len, squares }
    }

    pub fn vtx_data(&self, transform: &Transform3f) -> Vec<f32> {
        let mut vertices = vec![];
        for sq in &self.squares {
            vertices.extend_from_slice(&sq.vtx_data(transform));
        }
        vertices
    }

    pub fn vertices(&self, transform: &Matrix4f) -> Vec<Point3f> {
        let mut vertices = vec![];
        for sq in &self.squares {
            vertices.extend(
                sq.vertices()
                    .iter()
                    .map(|v| Point3f::from_homogeneous(transform * v.to_homogeneous()).unwrap()),
            );
        }
        vertices
    }
}
