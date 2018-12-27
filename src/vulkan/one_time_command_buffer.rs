use crate::vulkan::error::VulkanResult;
use ash::{version::DeviceV1_0, vk, Device};

pub struct OneTimeCommandBuffer<'a> {
    pub command_buffer: vk::CommandBuffer,
    device: &'a Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
}

impl<'a> OneTimeCommandBuffer<'a> {
    pub fn new(
        device: &Device,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
    ) -> VulkanResult<OneTimeCommandBuffer> {
        unsafe {
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_pool(command_pool)
                .command_buffer_count(1)
                .build();
            let command_buffer = device.allocate_command_buffers(&command_buffer_alloc_info)?[0];
            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                .build();
            device.begin_command_buffer(command_buffer, &command_buffer_begin_info)?;
            Ok(OneTimeCommandBuffer {
                command_buffer,
                device,
                queue,
                command_pool,
            })
        }
    }

    pub fn submit(&self, signal_semaphores: &[vk::Semaphore]) -> VulkanResult<()> {
        unsafe {
            self.device.end_command_buffer(self.command_buffer)?;
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(&[self.command_buffer])
                .signal_semaphores(signal_semaphores)
                .build();
            let fence_ci = vk::FenceCreateInfo::builder().build();
            let fence = self.device.create_fence(&fence_ci, None)?;
            self.device
                .queue_submit(self.queue, &[submit_info], fence)?;
            self.device.wait_for_fences(&[fence], true, std::u64::MAX)?;
            self.device
                .free_command_buffers(self.command_pool, &[self.command_buffer]);
            self.device.destroy_fence(fence, None);
            Ok(())
        }
    }
}
