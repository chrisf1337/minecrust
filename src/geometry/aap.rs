use crate::{geometry::AABB, types::prelude::*, utils::f32};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// Axis-aligned plane
pub struct AAP {
    axis: Axis,
    intercept: f32,
}

impl AAP {
    pub fn new(axis: Axis, intercept: f32) -> AAP {
        AAP { axis, intercept }
    }

    pub fn intersects_aabb(self, aabb: &AABB) -> bool {
        let intercept_vec = Vector3f::from_point3f(self.intercept_pt());
        let aabb_points = aabb.points();
        let first_sign = Vector3f::from_point3f(aabb_points[0])
            .dot(&intercept_vec)
            .sign();
        for &aabb_point in &aabb_points[1..] {
            if Vector3f::from_point3f(aabb_point)
                .dot(&intercept_vec)
                .sign()
                != first_sign
            {
                return true;
            }
        }
        false
    }

    #[inline]
    fn intercept_pt(&self) -> Point3f {
        match self.axis {
            Axis::X => Point3f::new(self.intercept, 0.0, 0.0),
            Axis::Y => Point3f::new(0.0, self.intercept, 0.0),
            Axis::Z => Point3f::new(0.0, 0.0, self.intercept),
        }
    }
}
