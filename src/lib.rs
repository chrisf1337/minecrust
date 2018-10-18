#![feature(custom_attribute, duration_as_u128)]
#![allow(unknown_lints)]
#![warn(clippy::all)]

use nalgebra as na;

pub mod bvh;
pub mod camera;
pub mod debug;
pub mod ecs;
pub mod event_handlers;
pub mod geometry;
pub mod gl;
pub mod types;
pub mod utils;

pub use crate::types::*;
