use crate::geometry::{BoundingBox, PrimitiveGeometry};
use crate::types::{Point3f, Transform3f, Vecf};

/// A 2d rectangle along the xz plane.
pub struct Rectangle {
    pub width: f32,  // along x axis
    pub height: f32, // along z axis
    pub vertices: Vec<Point3f>,
    last_transform: Vecf,
    last_vtx_data: Vec<f32>,
    last_vertices: Vec<Point3f>,
}

impl Rectangle {
    pub fn new(width: f32, height: f32) -> Rectangle {
        Rectangle {
            width,
            height,
            vertices: vec![
                Point3f::new(-width / 2.0, 0.0, -height / 2.0),
                Point3f::new(-width / 2.0, 0.0, height / 2.0),
                Point3f::new(width / 2.0, 0.0, height / 2.0),
                Point3f::new(width / 2.0, 0.0, -height / 2.0),
            ],
            last_transform: Vecf(vec![]),
            last_vtx_data: vec![],
            last_vertices: vec![],
        }
    }
}

impl PrimitiveGeometry for Rectangle {
    fn vtx_data(&mut self, transform: &Transform3f) -> Vec<f32> {
        if self
            .last_transform
            .eq(&Vecf::from_slice(transform.to_homogeneous().as_slice()))
        {
            return self.last_vtx_data.clone();
        }
        let mut data = vec![];
        data.extend_from_slice((transform * self.vertices[0]).coords.as_slice());
        data.extend_from_slice(&[0.0, 1.0]);
        data.extend_from_slice((transform * self.vertices[1]).coords.as_slice());
        data.extend_from_slice(&[0.0, 0.0]);
        data.extend_from_slice((transform * self.vertices[2]).coords.as_slice());
        data.extend_from_slice(&[1.0, 0.0]);

        data.extend_from_slice((transform * self.vertices[0]).coords.as_slice());
        data.extend_from_slice(&[0.0, 1.0]);
        data.extend_from_slice((transform * self.vertices[2]).coords.as_slice());
        data.extend_from_slice(&[1.0, 0.0]);
        data.extend_from_slice((transform * self.vertices[3]).coords.as_slice());
        data.extend_from_slice(&[1.0, 1.0]);

        self.last_vtx_data = data.clone();

        data
    }

    fn vertices(&mut self, transform: &Transform3f) -> Vec<Point3f> {
        if self
            .last_transform
            .eq(&Vecf::from_slice(transform.to_homogeneous().as_slice()))
        {
            return self.last_vertices.clone();
        }
        let vtxs: Vec<Point3f> = self.vertices.iter().map(|&v| transform * v).collect();
        self.last_vertices = vtxs.clone();
        vtxs
    }

    fn bounding_box(&self, transform: &Transform3f) -> BoundingBox {
        let min = transform * self.vertices[0];
        let max = transform * self.vertices[3];
        BoundingBox::new(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alga::general::SubsetOf;
    use crate::na::geometry::Isometry;
    use crate::na::{Rotation3, Translation3};
    use crate::types::Vector3f;
    use crate::utils;

    #[test]
    fn test_transform1() {
        let mut rect = Rectangle::new(1.0, 2.0);
        let t = Translation3::from_vector(Vector3f::new(0.0, 2.0, 0.0));
        let vertices = rect.vertices(&t.to_superset());
        assert!(utils::pt3f::almost_eq(
            &vertices[0],
            &Point3f::new(-0.5, 2.0, -1.0)
        ));
        assert!(utils::pt3f::almost_eq(
            &vertices[1],
            &Point3f::new(-0.5, 2.0, 1.0)
        ));
        assert!(utils::pt3f::almost_eq(
            &vertices[2],
            &Point3f::new(0.5, 2.0, 1.0)
        ));
        assert!(utils::pt3f::almost_eq(
            &vertices[3],
            &Point3f::new(0.5, 2.0, -1.0)
        ));
    }

    #[test]
    fn test_transform2() {
        let mut rect = Rectangle::new(1.0, 2.0);
        let t = Isometry::from_parts(
            Translation3::from_vector(Vector3f::new(0.0, 0.0, 0.5)),
            Rotation3::from_axis_angle(&Vector3f::x_axis(), ::std::f32::consts::FRAC_PI_2),
        );
        let vertices = rect.vertices(&t.to_superset());
        assert!(utils::pt3f::almost_eq(
            &vertices[0],
            &Point3f::new(-0.5, 1.0, 0.5)
        ));
        assert!(utils::pt3f::almost_eq(
            &vertices[1],
            &Point3f::new(-0.5, -1.0, 0.5)
        ));
        assert!(utils::pt3f::almost_eq(
            &vertices[2],
            &Point3f::new(0.5, -1.0, 0.5)
        ));
        assert!(utils::pt3f::almost_eq(
            &vertices[3],
            &Point3f::new(0.5, 1.0, 0.5)
        ));
    }
}
