use crate::{geometry::Aabb, types::prelude::*, utils::f32};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// Axis-aligned plane
pub struct AAP {
    /// Axis along which the normal is aligned
    axis: Axis,
    intercept: f32,
}

impl AAP {
    pub fn new(axis: Axis, intercept: f32) -> AAP {
        AAP { axis, intercept }
    }

    pub fn intersects_aabb(&self, aabb: &Aabb) -> bool {
        let intercept_pt = self.intercept_pt();
        let normal = match self.axis {
            Axis::X => Vector3f::x_axis(),
            Axis::Y => Vector3f::y_axis(),
            Axis::Z => Vector3f::z_axis(),
        };
        let aabb_points = aabb.points();
        let first_sign = (aabb_points[0] - intercept_pt).dot(&normal).sign();
        for &aabb_point in &aabb_points[1..] {
            if (aabb_point - intercept_pt).dot(&normal).sign() != first_sign {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intersects_aabb() {
        let aap = AAP::new(Axis::X, 2.0);
        assert!(aap.intersects_aabb(&Aabb::new_min_max(
            Point3f::new(1.0, 0.0, 0.0),
            Point3f::new(3.0, 1.0, 1.0)
        )));
        assert!(!aap.intersects_aabb(&Aabb::new_min_max(
            Point3f::new(1.0, 0.0, 0.0),
            Point3f::new(1.5, 1.0, 1.0)
        )));

        let aap = AAP::new(Axis::X, -2.0);
        assert!(aap.intersects_aabb(&Aabb::new_min_max(
            Point3f::new(-1.0, 0.0, 0.0),
            Point3f::new(-3.0, 1.0, 1.0)
        )));
        assert!(!aap.intersects_aabb(&Aabb::new_min_max(
            Point3f::new(-1.0, 0.0, 0.0),
            Point3f::new(-1.5, 1.0, 1.0)
        )));
    }
}
