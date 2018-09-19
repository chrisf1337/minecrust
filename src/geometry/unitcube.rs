use geometry::square::Square;
use na::{Isometry, Rotation3, Translation3};
use std::f32::consts::{FRAC_PI_2, PI};
use types::*;

pub struct UnitCube {
    side_len: f32,
    transform: Matrix4f,
    squares: Vec<Square>,
}

impl UnitCube {
    pub fn new(side_len: f32) -> UnitCube {
        let squares = vec![
            // top
            Square::new_with_transform(
                side_len,
                &Translation3::from_vector(Vector3f::new(0.0, side_len / 2.0, 0.0))
                    .to_homogeneous(),
            ),
            // front
            Square::new_with_transform(
                side_len,
                &Isometry::from_parts(
                    Translation3::from_vector(Vector3f::new(0.0, 0.0, side_len / 2.0)),
                    Rotation3::from_axis_angle(&Vector3f::x_axis(), FRAC_PI_2),
                ).to_homogeneous(),
            ),
            // right
            Square::new_with_transform(
                side_len,
                &Isometry::from_parts(
                    Translation3::from_vector(Vector3f::new(side_len / 2.0, 0.0, 0.0)),
                    Rotation3::from_axis_angle(&Vector3f::z_axis(), -FRAC_PI_2),
                ).to_homogeneous(),
            ),
            // back
            Square::new_with_transform(
                side_len,
                &Isometry::from_parts(
                    Translation3::from_vector(Vector3f::new(0.0, 0.0, -side_len / 2.0)),
                    Rotation3::from_axis_angle(&Vector3f::x_axis(), -FRAC_PI_2),
                ).to_homogeneous(),
            ),
            // left
            Square::new_with_transform(
                side_len,
                &Isometry::from_parts(
                    Translation3::from_vector(Vector3f::new(-side_len / 2.0, 0.0, 0.0)),
                    Rotation3::from_axis_angle(&Vector3f::z_axis(), FRAC_PI_2),
                ).to_homogeneous(),
            ),
            // bottom
            Square::new_with_transform(
                side_len,
                &Isometry::from_parts(
                    Translation3::from_vector(Vector3f::new(0.0, -side_len / 2.0, 0.0)),
                    Rotation3::from_axis_angle(&Vector3f::x_axis(), PI),
                ).to_homogeneous(),
            ),
        ];
        UnitCube {
            side_len,
            transform: Matrix4f::identity(),
            squares,
        }
    }

    pub fn transform(&mut self, transform: &Matrix4f) {
        self.transform = transform * self.transform;
    }

    pub fn vtx_data(&self) -> Vec<f32> {
        let mut vertices = vec![];
        for sq in &self.squares {
            vertices.extend_from_slice(&sq.vtx_data(&self.transform));
        }
        vertices
    }

    pub fn vertices(&self) -> Vec<Point3f> {
        let mut vertices = vec![];
        for sq in &self.squares {
            vertices.extend(
                sq.vertices().iter().map(|v| {
                    Point3f::from_homogeneous(self.transform * v.to_homogeneous()).unwrap()
                }),
            );
        }
        vertices
    }
}
