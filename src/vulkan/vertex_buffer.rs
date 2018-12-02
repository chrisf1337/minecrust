use crate::vulkan::{VulkanBase, VulkanResult};
use ash::vk;

#[derive(Debug, Copy, Clone)]
pub struct VertexBuffer {
    pub buf: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub capacity: vk::DeviceSize,
}

impl VertexBuffer {
    pub fn new(base: &VulkanBase, capacity: vk::DeviceSize) -> VulkanResult<VertexBuffer> {
        unsafe {
            let (buf, memory) = base.create_buffer(
                capacity,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?;
            Ok(VertexBuffer {
                buf,
                memory,
                capacity,
            })
        }
    }
}
