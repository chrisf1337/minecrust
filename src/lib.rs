#![feature(tool_lints, custom_attribute)]
#![allow(unknown_lints)]
#![warn(clippy::all)]

extern crate gl;
extern crate nalgebra as na;

pub mod debug;
pub mod render;
pub mod square;
pub mod types;
pub mod unitcube;
mod utils;

pub use types::*;
