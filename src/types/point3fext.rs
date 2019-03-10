use crate::{types::prelude::*, utils::f32};

pub trait Point3fExt {
    fn almost_eq(&self, p: &Point3f) -> bool;
    fn is_infinite(&self) -> bool;
    fn almost_is_int(&self) -> bool;
}

impl Point3fExt for Point3f {
    fn almost_eq(&self, p: &Point3f) -> bool {
        f32::almost_eq(self.x, p.x) && f32::almost_eq(self.y, p.y) && f32::almost_eq(self.z, p.z)
    }

    fn is_infinite(&self) -> bool {
        self.x.is_infinite() || self.y.is_infinite() || self.z.is_infinite()
    }

    fn almost_is_int(&self) -> bool {
        self.x.almost_is_int() && self.y.almost_is_int() && self.z.almost_is_int()
    }
}
