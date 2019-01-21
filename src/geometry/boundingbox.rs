use crate::{
    types::prelude::*,
    utils::{f32, pt3f},
};
use std::f32::INFINITY;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Face {
    Top,
    Front,
    Left,
    Right,
    Back,
    Bottom,
}

#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub min: Point3f,
    pub max: Point3f,
}

impl BoundingBox {
    pub fn new(min: Point3f, max: Point3f) -> BoundingBox {
        BoundingBox {
            min: pt3f::min(&min, &max),
            max: pt3f::max(&min, &max),
        }
    }

    fn _merge_two_bboxes(a: &BoundingBox, b: &BoundingBox) -> BoundingBox {
        BoundingBox {
            min: pt3f::min(&a.min, &b.min),
            max: pt3f::max(&a.max, &b.max),
        }
    }

    pub fn merge_bboxes(bboxes: &[BoundingBox]) -> BoundingBox {
        if bboxes.len() == 1 {
            bboxes[0]
        } else {
            let mut bbox = bboxes[0];
            for bb in &bboxes[0..] {
                bbox = BoundingBox::_merge_two_bboxes(&bbox, bb);
            }
            bbox
        }
    }

    pub fn new_infinite() -> BoundingBox {
        BoundingBox::new(
            Point3f::new(-INFINITY, -INFINITY, -INFINITY),
            Point3f::new(INFINITY, INFINITY, INFINITY),
        )
    }

    pub fn transform(&self, t: &Transform3f) -> BoundingBox {
        BoundingBox::new(t * self.min, t * self.max)
    }

    pub fn face(&self, point: &Point3f) -> Option<Face> {
        if f32::almost_eq(Vector3f::y_axis().dot(&(point - self.max)), 0.0) {
            Some(Face::Top)
        } else if f32::almost_eq(-Vector3f::y_axis().dot(&(point - self.min)), 0.0) {
            Some(Face::Bottom)
        } else if f32::almost_eq(Vector3f::z_axis().dot(&(point - self.max)), 0.0) {
            Some(Face::Front)
        } else if f32::almost_eq(-Vector3f::z_axis().dot(&(point - self.min)), 0.0) {
            Some(Face::Back)
        } else if f32::almost_eq(-Vector3f::x_axis().dot(&(point - self.min)), 0.0) {
            Some(Face::Left)
        } else if f32::almost_eq(Vector3f::x_axis().dot(&(point - self.max)), 0.0) {
            Some(Face::Right)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_face() {
        let bbox = BoundingBox::new(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));
        assert_eq!(bbox.face(&Point3f::new(0.0, 1.0, 0.0)), Some(Face::Top));
        assert_eq!(bbox.face(&Point3f::new(0.0, -1.0, 0.0)), Some(Face::Bottom));
        assert_eq!(bbox.face(&Point3f::new(-1.0, 0.5, 0.5)), Some(Face::Left));
        assert_eq!(bbox.face(&Point3f::new(1.0, 0.5, 0.5)), Some(Face::Right));
        assert_eq!(bbox.face(&Point3f::new(0.5, 0.5, 1.0)), Some(Face::Front));
        assert_eq!(bbox.face(&Point3f::new(-0.5, 0.5, -1.0)), Some(Face::Back));
    }
}
