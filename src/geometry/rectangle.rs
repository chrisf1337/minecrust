use crate::{
    geometry::{Aabb, PrimitiveGeometry},
    types::prelude::*,
    vulkan::Vertex3f,
};

/// A 2d rectangle along the xz plane.
#[derive(Clone, Debug)]
pub struct Rectangle {
    pub width: f32,  // along x axis
    pub height: f32, // along z axis
}

impl Rectangle {
    pub fn new(width: f32, height: f32) -> Rectangle {
        Rectangle { width, height }
    }

    fn vertices(&self) -> Vec<Vertex3f> {
        let width = self.width;
        let height = self.height;
        vec![
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
        ]
    }
}

impl PrimitiveGeometry for Rectangle {
    fn vtx_data(&self, transform: &Transform3f) -> Vec<Vertex3f> {
        let vertices = self.vertices();
        vec![0, 1, 2, 0, 2, 3]
            .into_iter()
            .map(|i| vertices[i].transform(transform))
            .collect()
    }

    fn vtx_pts(&self, transform: &Transform3f) -> Vec<Point3f> {
        self.vertices()
            .iter()
            .map(|v| v.transform(transform).pos)
            .collect()
    }

    fn aabb(&self, transform: &Transform3f) -> Aabb {
        let vertices = self.vertices();
        let min = transform * vertices[0].pos;
        let max = transform * vertices[3].pos;
        Aabb::new_min_max(min, max)
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
        let vertices = rect.vtx_pts(&t.to_superset());
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
        let vertices = rect.vtx_pts(&t.to_superset());
        assert!(vertices[0].almost_eq(&Point3f::new(-0.5, 1.0, 0.5)));
        assert!(vertices[1].almost_eq(&Point3f::new(-0.5, -1.0, 0.5)));
        assert!(vertices[2].almost_eq(&Point3f::new(0.5, -1.0, 0.5)));
        assert!(vertices[3].almost_eq(&Point3f::new(0.5, 1.0, 0.5)));
    }
}
