#![feature(custom_attribute, try_trait)]
#![allow(unknown_lints)]
#![warn(clippy::all)]
#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::cyclomatic_complexity
)]

use nalgebra as na;

#[macro_use]
mod utils;
mod camera;
pub mod ecs;
mod event_handlers;
pub mod game;
pub mod geometry;
pub mod octree;
pub mod renderer;
pub mod types;
pub mod vulkan;

pub use crate::types::prelude::*;
