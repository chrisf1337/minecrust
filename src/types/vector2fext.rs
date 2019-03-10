use crate::types::prelude::*;

pub trait Vector2fExt {
    fn almost_is_int(&self) -> bool;
}

impl Vector2fExt for Vector2f {
    fn almost_is_int(&self) -> bool {
        self.x.almost_is_int() && self.y.almost_is_int()
    }
}
