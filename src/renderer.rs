use crate::{game::GameState, vulkan::VulkanError};
use ash::vk;
use failure_derive::Fail;
use winit::{EventsLoop, Window};

#[derive(Fail, Debug)]
pub enum RendererError {
    #[fail(display = "{:?}", _0)]
    VulkanError(#[cause] VulkanError),
}

impl From<VulkanError> for RendererError {
    fn from(err: VulkanError) -> RendererError {
        RendererError::VulkanError(err)
    }
}

impl From<vk::Result> for RendererError {
    fn from(err: vk::Result) -> RendererError {
        RendererError::VulkanError(err.into())
    }
}

pub type RendererResult<T> = Result<T, RendererError>;

pub trait Renderer {
    unsafe fn draw_frame(&mut self, state: &GameState, resized: bool) -> RendererResult<()>;
    fn events_loop(&mut self) -> &mut EventsLoop;
    fn window(&self) -> &Window;
}
