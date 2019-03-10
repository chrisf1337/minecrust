use crate::types::prelude::*;

pub trait Point2fExt {
    fn almost_is_int(&self) -> bool;
}

impl Point2fExt for Point2f {
    fn almost_is_int(&self) -> bool {
        self.x.almost_is_int() && self.y.almost_is_int()
    }
}
