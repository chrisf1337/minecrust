use alga::general::SubsetOf;
use crate::geometry::{square::Square, BoundingBox, PrimitiveGeometry};
use crate::na::{Isometry, Rotation3, Translation3};
use crate::types::*;
use std::f32::consts::{FRAC_PI_2, PI};

pub struct UnitCube {
    pub side_len: f32,
    transforms: Vec<Transform3f>,
    last_transform: Vecf,
    last_vtx_data: Vec<f32>,
    last_vertices: Vec<Point3f>,
}

impl UnitCube {
    pub fn new(side_len: f32) -> UnitCube {
        let transforms = vec![
            // top
            Translation3::from_vector(Vector3f::new(0.0, side_len / 2.0, 0.0)).to_superset(),
            Isometry::from_parts(
                Translation3::from_vector(Vector3f::new(0.0, 0.0, side_len / 2.0)),
                Rotation3::from_axis_angle(&Vector3f::x_axis(), FRAC_PI_2),
            )
            .to_superset(),
            Isometry::from_parts(
                Translation3::from_vector(Vector3f::new(side_len / 2.0, 0.0, 0.0)),
                Rotation3::from_axis_angle(&Vector3f::z_axis(), -FRAC_PI_2),
            )
            .to_superset(),
            Isometry::from_parts(
                Translation3::from_vector(Vector3f::new(0.0, 0.0, -side_len / 2.0)),
                Rotation3::from_axis_angle(&Vector3f::x_axis(), -FRAC_PI_2),
            )
            .to_superset(),
            Isometry::from_parts(
                Translation3::from_vector(Vector3f::new(-side_len / 2.0, 0.0, 0.0)),
                Rotation3::from_axis_angle(&Vector3f::z_axis(), FRAC_PI_2),
            )
            .to_superset(),
            Isometry::from_parts(
                Translation3::from_vector(Vector3f::new(0.0, -side_len / 2.0, 0.0)),
                Rotation3::from_axis_angle(&Vector3f::x_axis(), PI),
            )
            .to_superset(),
        ];
        UnitCube {
            side_len,
            transforms,
            last_transform: Vecf(vec![]),
            last_vtx_data: vec![],
            last_vertices: vec![],
        }
    }
}

impl PrimitiveGeometry for UnitCube {
    fn vtx_data(&mut self, transform: &Transform3f) -> Vec<f32> {
        if self
            .last_transform
            .eq(&Vecf::from_slice(transform.to_homogeneous().as_slice()))
        {
            return self.last_vtx_data.clone();
        }
        let mut sq = Square::new(self.side_len);
        let mut vertices = vec![];
        for tr in &self.transforms {
            vertices.extend_from_slice(&sq.vtx_data(&(transform * tr)));
        }
        self.last_vtx_data = vertices.clone();
        vertices
    }

    fn vertices(&mut self, transform: &Transform3f) -> Vec<Point3f> {
        if self
            .last_transform
            .eq(&Vecf::from_slice(transform.to_homogeneous().as_slice()))
        {
            return self.last_vertices.clone();
        }
        let mut sq = Square::new(self.side_len);
        let mut vertices = vec![];
        for tr in &self.transforms {
            vertices.extend(sq.vertices(&(transform * tr)));
        }
        self.last_vertices = vertices.clone();
        vertices
    }

    fn bounding_box(&self, transform: &Transform3f) -> BoundingBox {
        let min = transform * Point3f::new(-self.side_len, -self.side_len, -self.side_len);
        let max = transform * Point3f::new(self.side_len, self.side_len, self.side_len);
        BoundingBox::new(min, max)
    }
}
