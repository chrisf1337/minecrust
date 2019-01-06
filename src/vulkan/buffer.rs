use crate::vulkan::VulkanCore;
use ash::{prelude::VkResult, version::DeviceV1_0, vk, Device};
use std::{fmt, marker::PhantomData, rc::Rc};

#[derive(Clone)]
pub struct Buffer<T> {
    phantom_ty: PhantomData<T>,
    device: Rc<Device>,
    buffer: Option<vk::Buffer>,
    memory: Option<vk::DeviceMemory>,
    pub capacity: vk::DeviceSize,
    /// Number of elements in the buffer.
    pub len: usize,
    /// `buf_len` = `len` * `size_of::<T>()`
    pub buf_len: vk::DeviceSize,

    pub ptr: *mut T,

    pub usage: vk::BufferUsageFlags,
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
    ) -> Buffer<T> {
        Buffer {
            phantom_ty: PhantomData,
            device: core.device.clone(),
            buffer: None,
            memory: None,
            capacity,
            buf_len: 0,
            len: 0,

            ptr: std::ptr::null_mut(),
            usage,
            memory_property_flags,
        }
    }

    pub fn new_init(
        core: &VulkanCore,
        capacity: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> VkResult<Buffer<T>> {
        let mut buf = Buffer::new(core, capacity, usage, memory_property_flags);
        buf.init(core)?;
        Ok(buf)
    }

    pub fn init(&mut self, core: &VulkanCore) -> VkResult<()> {
        let (buffer, memory) =
            unsafe { core.create_buffer(self.capacity, self.usage, self.memory_property_flags)? };
        self.buffer = Some(buffer);
        self.memory = Some(memory);
        Ok(())
    }

    /// Copies all elements in `ts` into the buffer. Also calls `flush_mapped_memory_ranges()` if
    /// the buffer's `memory_property_flags` don't contain `HOST_COHERENT`.
    pub fn copy_data(&mut self, ts: &[T]) -> VkResult<()> {
        assert!((ts.len() * std::mem::size_of::<T>()) as vk::DeviceSize <= self.capacity);
        assert!(!self.ptr.is_null());

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
                        .memory(self.memory.unwrap())
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
                self.memory.unwrap(),
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
            self.device.unmap_memory(self.memory.unwrap());
            self.ptr = std::ptr::null_mut();
        }
    }

    pub fn buffer(&self) -> vk::Buffer {
        self.buffer.unwrap()
    }

    pub fn deinit(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                self.unmap();
            }
            if let Some(memory) = self.memory {
                self.device.free_memory(memory, None);
            }
            if let Some(buffer) = self.buffer {
                self.device.destroy_buffer(buffer, None);
            }
        }
    }
}
