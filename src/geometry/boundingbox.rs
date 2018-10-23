use crate::geometry::unitcube::UnitCube;
use crate::types::*;
use crate::utils::pt3f;

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

    pub fn merge(a: &BoundingBox, b: &BoundingBox) -> BoundingBox {
        BoundingBox {
            min: pt3f::min(&a.min, &b.min),
            max: pt3f::max(&a.max, &b.max),
        }
    }
}
