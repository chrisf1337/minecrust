use crate::vulkan::VulkanCore;
use ash::{prelude::VkResult, version::DeviceV1_0, vk, Device};
use std::{fmt, marker::PhantomData, rc::Rc};

#[derive(Clone)]
pub struct Buffer<T> {
    phantom_ty: PhantomData<T>,
    device: Rc<Device>,
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub capacity: vk::DeviceSize,
    /// Number of elements in the buffer.
    pub len: usize,
    /// `buf_len` = `len` * `size_of::<T>()`
    pub buf_len: vk::DeviceSize,

    pub ptr: *mut T,

    pub memory_property_flags: vk::MemoryPropertyFlags,
}

impl<T> fmt::Debug for Buffer<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Buffer")
            .field("buf", &self.buffer)
            .field("memory", &self.memory)
            .field("len", &self.len)
            .field("buf_len", &self.buf_len)
            .field("capacity", &self.capacity)
            .field("ptr", &self.ptr)
            .finish()
    }
}

impl<T> Buffer<T> {
    pub fn new(
        core: &VulkanCore,
        capacity: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> VkResult<Buffer<T>> {
        unsafe {
            let (buffer, memory) = core.create_buffer(capacity, usage, memory_property_flags)?;
            Ok(Buffer {
                phantom_ty: PhantomData,
                device: core.device.clone(),
                buffer,
                memory,
                capacity,
                buf_len: 0,
                len: 0,

                ptr: std::ptr::null_mut(),
                memory_property_flags,
            })
        }
    }

    /// Copies all elements in `ts` into the buffer. Also calls `flush_mapped_memory_ranges()` if
    /// the buffer's `memory_property_flags` don't contain `HOST_COHERENT`.
    pub fn copy_data(&mut self, ts: &[T]) -> VkResult<()> {
        assert!((ts.len() * std::mem::size_of::<T>()) as vk::DeviceSize <= self.capacity);

        unsafe {
            self.buf_len = (ts.len() * std::mem::size_of::<T>()) as vk::DeviceSize;
            self.len = ts.len();

            std::ptr::copy_nonoverlapping(ts.as_ptr(), self.ptr, ts.len());

            if !self
                .memory_property_flags
                .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
            {
                self.device
                    .flush_mapped_memory_ranges(&[vk::MappedMemoryRange::builder()
                        .memory(self.memory)
                        .offset(0)
                        .size(vk::WHOLE_SIZE)
                        .build()])?;
            }

            Ok(())
        }
    }

    pub fn map(&mut self) -> VkResult<*mut T> {
        assert!(self
            .memory_property_flags
            .contains(vk::MemoryPropertyFlags::HOST_VISIBLE));
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
        assert!(!self.ptr.is_null());
        unsafe {
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
