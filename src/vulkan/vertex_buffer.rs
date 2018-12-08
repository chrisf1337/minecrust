use crate::vulkan::{VulkanBase, VulkanResult};
use ash::{version::DeviceV1_0, vk, Device};
use std::{fmt, rc::Rc};

#[derive(Clone)]
pub struct VertexBuffer {
    device: Rc<Device>,
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub capacity: vk::DeviceSize,
}

impl fmt::Debug for VertexBuffer {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("VertexBuffer")
            .field("buf", &self.buffer)
            .field("memory", &self.memory)
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl VertexBuffer {
    pub fn new(base: &VulkanBase, capacity: vk::DeviceSize) -> VulkanResult<VertexBuffer> {
        unsafe {
            let (buffer, memory) = base.create_buffer(
                capacity,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?;
            Ok(VertexBuffer {
                device: base.device.clone(),
                buffer,
                memory,
                capacity,
            })
        }
    }
}

impl Drop for VertexBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.memory, None);
            self.device.destroy_buffer(self.buffer, None);
        }
    }
}
