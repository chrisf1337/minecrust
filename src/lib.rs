#![feature(tool_lints, custom_attribute)]
#![allow(unknown_lints)]
#![warn(clippy::all)]

extern crate failure;
extern crate gl;
extern crate glutin;
extern crate nalgebra as na;

pub mod camera;
pub mod debug;
pub mod event_handlers;
pub mod geometry;
pub mod render;
pub mod types;
pub mod utils;

pub use types::*;
