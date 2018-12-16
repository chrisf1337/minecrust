use crate::{game::GameState, types::*, vulkan::error::VulkanError};
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
    fn draw_frame(
        &mut self,
        state: &GameState,
        render_data: &RenderData,
        resized: bool,
    ) -> RendererResult<()>;
    fn events_loop(&mut self) -> &mut EventsLoop;
    fn window(&self) -> &Window;
}

pub struct RenderData {
    pub vertices: Vec<Vertex3f>,
}
