use crate::vulkan::VulkanCore;
use ash::{version::DeviceV1_0, vk};
use std::{fmt, rc::Rc};

#[derive(Clone)]
pub struct Texture {
    device: Rc<ash::Device>,
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
}

impl fmt::Debug for Texture {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Texture")
            .field("image", &self.image)
            .field("memory", &self.memory)
            .field("view", &self.view)
            .finish()
    }
}

impl Texture {
    pub fn new(
        core: &VulkanCore,
        image: vk::Image,
        memory: vk::DeviceMemory,
        view: vk::ImageView,
    ) -> Texture {
        Texture {
            device: core.device.clone(),
            image,
            memory,
            view,
        }
    }

    pub fn deinit(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.view, None);
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);
        }
    }
}
