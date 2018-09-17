#![feature(tool_lints, custom_attribute)]
#![allow(unknown_lints)]
#![warn(clippy::all)]

extern crate gl;
extern crate glutin;
extern crate nalgebra as na;

pub mod camera;
pub mod debug;
pub mod event_handlers;
pub mod render;
pub mod square;
pub mod types;
pub mod unitcube;
pub mod utils;

use na::Unit;
pub use types::*;
