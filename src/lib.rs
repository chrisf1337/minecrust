#![feature(custom_attribute, duration_as_u128, try_trait)]
#![allow(unknown_lints)]
#![warn(clippy::all)]
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use nalgebra as na;

#[macro_use]
pub mod utils;
pub mod bvh;
pub mod camera;
pub mod debug;
pub mod ecs;
pub mod event_handlers;
pub mod geometry;
pub mod gl;
pub mod types;
pub mod vulkan;

pub use crate::types::*;
