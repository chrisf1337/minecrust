use crate::vulkan::{VulkanBase, VulkanResult};
use ash::{version::DeviceV1_0, vk, Device};
use std::{fmt, marker::PhantomData, rc::Rc};

#[derive(Clone)]
pub struct VertexBuffer<T> {
    phantom_ty: PhantomData<T>,
    device: Rc<Device>,
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub capacity: vk::DeviceSize,
    pub len: vk::DeviceSize,
}

impl<T> fmt::Debug for VertexBuffer<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("VertexBuffer")
            .field("buf", &self.buffer)
            .field("memory", &self.memory)
            .field("len", &self.len)
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl<T> VertexBuffer<T> {
    pub fn new(
        base: &VulkanBase,
        capacity: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_properties: vk::MemoryPropertyFlags,
    ) -> VulkanResult<VertexBuffer<T>> {
        unsafe {
            let (buffer, memory) = base.create_buffer(capacity, usage, memory_properties)?;
            Ok(VertexBuffer {
                phantom_ty: PhantomData,
                device: base.device.clone(),
                buffer,
                memory,
                capacity,
                len: 0,
            })
        }
    }

    pub fn copy_data(&mut self, ts: &[T]) -> VulkanResult<()> {
        unsafe {
            self.len = (ts.len() * std::mem::size_of::<T>()) as vk::DeviceSize;
            let data =
                self.device
                    .map_memory(self.memory, 0, self.len, vk::MemoryMapFlags::empty())?
                    as *mut T;
            std::ptr::copy_nonoverlapping(ts.as_ptr(), data, ts.len());
            self.device.unmap_memory(self.memory);

            Ok(())
        }
    }
}

impl<T> Drop for VertexBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.memory, None);
            self.device.destroy_buffer(self.buffer, None);
        }
    }
}
