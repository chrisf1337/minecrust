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
    pub layout: vk::DescriptorSetLayout,
    pub bindings: Vec<DescriptorSetLayoutBinding>,
}

impl DescriptorSetLayout {
    pub fn new(
        core: &VulkanCore,
        bindings: Vec<DescriptorSetLayoutBinding>,
    ) -> VkResult<DescriptorSetLayout> {
        unsafe {
            let vk_bindings = bindings.iter().map(|l| l.binding).collect::<Vec<_>>();
            let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&vk_bindings);
            let layout = core
                .device
                .create_descriptor_set_layout(&create_info, None)?;
            Ok(DescriptorSetLayout { layout, bindings })
        }
    }

    pub fn deinit(&self, core: &VulkanCore) {
        unsafe {
            core.device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}

pub struct DescriptorSet {}
