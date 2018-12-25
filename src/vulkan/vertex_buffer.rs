use crate::vulkan::{VulkanBase, VulkanResult};
use ash::{prelude::VkResult, version::DeviceV1_0, vk, Device};
use std::{fmt, marker::PhantomData, rc::Rc};

#[derive(Clone)]
pub struct VertexBuffer<T> {
    phantom_ty: PhantomData<T>,
    device: Rc<Device>,
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub capacity: vk::DeviceSize,
    pub len: usize,
    pub buf_len: vk::DeviceSize,

    pub ptr: *mut T,
}

impl<T> fmt::Debug for VertexBuffer<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("VertexBuffer")
            .field("buf", &self.buffer)
            .field("memory", &self.memory)
            .field("len", &self.len)
            .field("buf_len", &self.buf_len)
            .field("capacity", &self.capacity)
            .field("ptr", &self.ptr)
            .finish()
    }
}

impl<T> VertexBuffer<T> {
    pub fn new(
        base: &VulkanBase,
        capacity: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_properties: vk::MemoryPropertyFlags,
    ) -> VkResult<VertexBuffer<T>> {
        unsafe {
            let (buffer, memory) = base.create_buffer(capacity, usage, memory_properties)?;
            Ok(VertexBuffer {
                phantom_ty: PhantomData,
                device: base.device.clone(),
                buffer,
                memory,
                capacity,
                buf_len: 0,
                len: 0,

                ptr: std::ptr::null_mut(),
            })
        }
    }

    pub fn copy_data(&mut self, ts: &[T]) {
        unsafe {
            self.buf_len = (ts.len() * std::mem::size_of::<T>()) as vk::DeviceSize;
            self.len = ts.len();

            std::ptr::copy_nonoverlapping(ts.as_ptr(), self.ptr, ts.len());
        }
    }

    pub fn map(&mut self) -> VkResult<*mut T> {
        unsafe {
            self.ptr = self.device.map_memory(
                self.memory,
                0,
                vk::WHOLE_SIZE,
                vk::MemoryMapFlags::empty(),
            )? as *mut T;
            Ok(self.ptr)
        }
    }

    pub fn unmap(&mut self) {
        unsafe {
            assert!(!self.ptr.is_null());
            self.device.unmap_memory(self.memory);
            self.ptr = std::ptr::null_mut();
        }
    }

    pub fn deinit(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                self.unmap();
            }
            self.device.free_memory(self.memory, None);
            self.device.destroy_buffer(self.buffer, None);
        }
    }
}
