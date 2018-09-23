use types::*;

pub struct BoundingBox {
    pub bounds: [Vector3f; 2],
}

impl BoundingBox {
    pub fn new(bounds: [Vector3f; 2]) -> BoundingBox {
        BoundingBox { bounds }
    }
}
