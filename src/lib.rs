#![feature(tool_lints, custom_attribute, duration_as_u128)]
#![allow(unknown_lints)]
#![warn(clippy::all)]

extern crate failure;
extern crate gl;
extern crate glutin;
extern crate nalgebra as na;
extern crate specs;
#[macro_use]
extern crate specs_derive;
extern crate num_traits;

pub mod bvh;
pub mod camera;
pub mod debug;
pub mod ecs;
pub mod event_handlers;
pub mod geometry;
pub mod render;
pub mod types;
pub mod utils;

pub use crate::types::*;
