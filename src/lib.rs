#![feature(tool_lints, custom_attribute)]
#![allow(unknown_lints)]
#![warn(clippy::all)]

extern crate gl;
extern crate glutin;
extern crate nalgebra as na;

pub mod debug;
pub mod event_handlers;
pub mod render;
pub mod square;
pub mod types;
pub mod unitcube;
mod utils;

use na::Unit;
pub use types::*;

pub struct Camera {
    pub pos: Point3f,
    pub target: Point3f,
    pub up: Vector3f,
}

impl Camera {
    pub fn new(pos: Point3f, target: Point3f, up: Vector3f) -> Camera {
        Camera { pos, target, up }
    }

    pub fn to_matrix(&self) -> Matrix4f {
        Matrix4f::look_at_rh(&self.pos, &self.target, &self.up)
    }

    pub fn look_at_dir(&self) -> Unit<Vector3f> {
        Unit::new_normalize(self.target - self.pos)
    }
}
