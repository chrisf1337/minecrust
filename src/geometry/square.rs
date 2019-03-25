use crate::{
    geometry::{rectangle::Rectangle, Aabb, PrimitiveGeometry},
    types::prelude::*,
    vulkan::Vertex3f,
};

#[derive(Clone, Debug)]
pub struct Square {
    rect: Rectangle,
}

impl Square {
    pub fn new(side: f32) -> Square {
        Square {
            rect: Rectangle::new(side, side),
        }
    }
}

impl PrimitiveGeometry for Square {
    fn vtx_data(&self, transform: &Transform3f) -> Vec<Vertex3f> {
        self.rect.vtx_data(transform)
    }

    fn vtx_pts(&self, transform: &Transform3f) -> Vec<Point3f> {
        self.rect.vtx_pts(transform)
    }

    fn aabb(&self, transform: &Transform3f) -> Aabb {
        self.rect.aabb(transform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::na::geometry::Isometry;
    use crate::na::{Rotation3, Translation3};
    use alga::general::SubsetOf;

    #[test]
    fn test_transform1() {
        let s = Square::new(1.0);
        let t = Translation3::from(Vector3f::new(0.0, 2.0, 0.0));
        let vertices = s.vtx_pts(&t.to_superset());
        assert!(vertices[0].almost_eq(&Point3f::new(-0.5, 2.0, -0.5)));
        assert!(vertices[1].almost_eq(&Point3f::new(-0.5, 2.0, 0.5)));
        assert!(vertices[2].almost_eq(&Point3f::new(0.5, 2.0, 0.5)));
        assert!(vertices[3].almost_eq(&Point3f::new(0.5, 2.0, -0.5)));
    }

    #[test]
    fn test_transform2() {
        let s = Square::new(1.0);
        let t = Isometry::from_parts(
            Translation3::from(Vector3f::new(0.0, 0.0, 0.5)),
            Rotation3::from_axis_angle(&Vector3f::x_axis(), ::std::f32::consts::FRAC_PI_2),
        );
        let vertices = s.vtx_pts(&t.to_superset());
        assert!(vertices[0].almost_eq(&Point3f::new(-0.5, 0.5, 0.5)));
        assert!(vertices[1].almost_eq(&Point3f::new(-0.5, -0.5, 0.5)));
        assert!(vertices[2].almost_eq(&Point3f::new(0.5, -0.5, 0.5)));
        assert!(vertices[3].almost_eq(&Point3f::new(0.5, 0.5, 0.5)));
    }
}
