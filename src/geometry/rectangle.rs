use crate::{
    geometry::{PrimitiveGeometry, AABB},
    types::prelude::*,
    vulkan::Vertex3f,
};

/// A 2d rectangle along the xz plane.
#[derive(Clone, Debug)]
pub struct Rectangle {
    pub width: f32,  // along x axis
    pub height: f32, // along z axis
    pub vertices: Vec<Vertex3f>,
}

impl Rectangle {
    pub fn new(width: f32, height: f32) -> Rectangle {
        Rectangle {
            width,
            height,
            vertices: vec![
                Vertex3f::new(
                    Point3f::new(-width / 2.0, 0.0, -height / 2.0),
                    Point2f::new(0.0, 0.0),
                ),
                Vertex3f::new(
                    Point3f::new(-width / 2.0, 0.0, height / 2.0),
                    Point2f::new(0.0, 1.0),
                ),
                Vertex3f::new(
                    Point3f::new(width / 2.0, 0.0, height / 2.0),
                    Point2f::new(1.0, 1.0),
                ),
                Vertex3f::new(
                    Point3f::new(width / 2.0, 0.0, -height / 2.0),
                    Point2f::new(1.0, 0.0),
                ),
            ],
        }
    }
}

impl PrimitiveGeometry for Rectangle {
    fn vtx_data(&self, transform: &Transform3f) -> Vec<Vertex3f> {
        vec![0, 1, 2, 0, 2, 3]
            .into_iter()
            .map(|i| self.vertices[i].transform(transform))
            .collect()
    }

    fn vertices(&self, transform: &Transform3f) -> Vec<Point3f> {
        self.vertices
            .iter()
            .map(|v| v.transform(transform).pos)
            .collect()
    }

    fn aabb(&self, transform: &Transform3f) -> AABB {
        let min = transform * self.vertices[0].pos;
        let max = transform * self.vertices[3].pos;
        AABB::new(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::na::{geometry::Isometry, Rotation3, Translation3};
    use alga::general::SubsetOf;

    #[test]
    fn test_transform1() {
        let rect = Rectangle::new(1.0, 2.0);
        let t = Translation3::from(Vector3f::new(0.0, 2.0, 0.0));
        let vertices = rect.vertices(&t.to_superset());
        assert!(vertices[0].almost_eq(&Point3f::new(-0.5, 2.0, -1.0)));
        assert!(vertices[1].almost_eq(&Point3f::new(-0.5, 2.0, 1.0)));
        assert!(vertices[2].almost_eq(&Point3f::new(0.5, 2.0, 1.0)));
        assert!(vertices[3].almost_eq(&Point3f::new(0.5, 2.0, -1.0)));
    }

    #[test]
    fn test_transform2() {
        let rect = Rectangle::new(1.0, 2.0);
        let t = Isometry::from_parts(
            Translation3::from(Vector3f::new(0.0, 0.0, 0.5)),
            Rotation3::from_axis_angle(&Vector3f::x_axis(), ::std::f32::consts::FRAC_PI_2),
        );
        let vertices = rect.vertices(&t.to_superset());
        assert!(vertices[0].almost_eq(&Point3f::new(-0.5, 1.0, 0.5)));
        assert!(vertices[1].almost_eq(&Point3f::new(-0.5, -1.0, 0.5)));
        assert!(vertices[2].almost_eq(&Point3f::new(0.5, -1.0, 0.5)));
        assert!(vertices[3].almost_eq(&Point3f::new(0.5, 1.0, 0.5)));
    }
}
