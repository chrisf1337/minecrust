use crate::geometry::unitcube::UnitCube;
use crate::types::*;
use crate::utils::{pt3f_max, pt3f_min};

pub struct BoundingBox {
    pub min: Point3f,
    pub max: Point3f,
}

impl BoundingBox {
    pub fn new(min: Point3f, max: Point3f) -> BoundingBox {
        BoundingBox {
            min: pt3f_min(&min, &max),
            max: pt3f_max(&min, &max),
        }
    }

    pub fn merge(a: &BoundingBox, b: &BoundingBox) -> BoundingBox {
        BoundingBox {
            min: pt3f_min(&a.min, &b.min),
            max: pt3f_max(&a.max, &b.max),
        }
    }
}
