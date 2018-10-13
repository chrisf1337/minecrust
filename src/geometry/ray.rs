use crate::geometry::boundingbox::BoundingBox;
use crate::types::*;
use std::f32;
use std::mem;

pub struct Ray {
    pub origin: Point3f,
    pub direction: Vector3f,
}

impl Ray {
    pub fn new(origin: Point3f, direction: Vector3f) -> Ray {
        Ray { origin, direction }
    }

    pub fn at(&self, t: f32) -> Point3f {
        self.origin + t * self.direction
    }

    pub fn intersect(&self, bbox: &BoundingBox) -> Option<Point3f> {
        let min = bbox.min;
        let max = bbox.max;

        let mut tmin = (min.x - self.origin.x) / self.direction.x;
        let mut tmax = (max.x - self.origin.x) / self.direction.x;
        if tmin > tmax {
            mem::swap(&mut tmin, &mut tmax);
        }

        let mut tymin = (min.y - self.origin.y) / self.direction.y;
        let mut tymax = (max.y - self.origin.y) / self.direction.y;
        if tymin > tymax {
            mem::swap(&mut tymin, &mut tymax);
        }

        if tmin > tymax || tymin > tmax {
            return None;
        }
        tmin = f32::max(tmin, tymin);
        tmax = f32::min(tmax, tymax);

        let mut tzmin = (min.z - self.origin.z) / self.direction.z;
        let mut tzmax = (max.z - self.origin.z) / self.direction.z;
        if tzmin > tzmax {
            mem::swap(&mut tzmin, &mut tzmax);
        }

        if tmin > tzmax || tzmin > tmax {
            return None;
        }
        tmin = f32::max(tmin, tzmin);
        tmax = f32::min(tmax, tzmax);

        if tmax < 0.0 {
            None
        } else if tmin < 0.0 {
            Some(self.at(tmax))
        } else {
            Some(self.at(tmin))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::pt3f_almost_eq;

    #[test]
    fn test_intersect1() {
        let r = Ray::new(Point3f::origin(), Vector3f::x());
        let bbox = BoundingBox::new(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));
        let intersection = r.intersect(&bbox);
        assert!(intersection.is_some());
        assert!(pt3f_almost_eq(
            &intersection.unwrap(),
            &Point3f::new(1.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn test_intersect2() {
        let r = Ray::new(Point3f::new(-1.0, -0.5, 2.0), Vector3f::new(1.0, 0.0, -1.0));
        let bbox = BoundingBox::new(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));
        let intersection = r.intersect(&bbox);
        assert!(intersection.is_some());
        println!("{:?}", intersection);
        assert!(pt3f_almost_eq(
            &intersection.unwrap(),
            &Point3f::new(0.0, -0.5, 1.0)
        ));
    }

    #[test]
    fn test_no_intersect() {
        let r = Ray::new(Point3f::new(2.0, 0.0, 0.0), Vector3f::x());
        let bbox = BoundingBox::new(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));
        let intersection = r.intersect(&bbox);
        assert!(intersection.is_none());
    }
}
