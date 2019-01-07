#![feature(custom_attribute, try_trait)]
#![allow(unknown_lints)]
#![warn(clippy::all)]
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use nalgebra as na;

#[macro_use]
mod utils;
mod bvh;
mod camera;
mod ecs;
mod event_handlers;
pub mod game;
mod geometry;
pub mod renderer;
mod types;
pub mod vulkan;

pub use crate::types::prelude::*;
