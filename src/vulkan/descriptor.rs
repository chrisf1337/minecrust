use crate::vulkan::VulkanCore;
use ash::{prelude::VkResult, version::DeviceV1_0, vk};

#[derive(Debug, Clone)]
pub struct DescriptorSetLayoutBinding {
    pub binding_index: u32,
    pub ty: vk::DescriptorType,
    count: u32,
    pub stage_flags: vk::ShaderStageFlags,
    pub immutable_samplers: Vec<vk::Sampler>,
    pub binding: vk::DescriptorSetLayoutBinding,
}

impl DescriptorSetLayoutBinding {
    pub fn new(
        binding_index: u32,
        ty: vk::DescriptorType,
        count: u32,
        stage_flags: vk::ShaderStageFlags,
        immutable_samplers: Vec<vk::Sampler>,
    ) -> DescriptorSetLayoutBinding {
        let mut binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(binding_index)
            .descriptor_type(ty)
            .descriptor_count(count)
            .stage_flags(stage_flags);
        if !immutable_samplers.is_empty() {
            assert_eq!(immutable_samplers.len(), count as usize);
            binding = binding.immutable_samplers(&immutable_samplers);
        }
        let binding = binding.build();
        DescriptorSetLayoutBinding {
            binding_index,
            ty,
            count,
            stage_flags,
            immutable_samplers,
            binding,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DescriptorSetLayout {
    pub layout: Option<vk::DescriptorSetLayout>,
    pub bindings: Vec<DescriptorSetLayoutBinding>,
}

impl DescriptorSetLayout {
    pub fn new(bindings: Vec<DescriptorSetLayoutBinding>) -> VkResult<DescriptorSetLayout> {
        Ok(DescriptorSetLayout {
            layout: None,
            bindings,
        })
    }

    pub fn init(&mut self, core: &VulkanCore) -> VkResult<()> {
        let vk_bindings = self.bindings.iter().map(|l| l.binding).collect::<Vec<_>>();
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&vk_bindings);
        unsafe {
            self.layout = Some(
                core.device
                    .create_descriptor_set_layout(&create_info, None)?,
            );
        }
        Ok(())
    }

    pub fn layout(&self) -> vk::DescriptorSetLayout {
        self.layout.unwrap()
    }

    pub fn deinit(&self, core: &VulkanCore) {
        unsafe {
            if let Some(layout) = self.layout {
                core.device.destroy_descriptor_set_layout(layout, None);
            }
        }
    }
}
