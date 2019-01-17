use ash::vk;

pub trait VertexInput {
    fn binding_description() -> vk::VertexInputBindingDescription;
    fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription>;
}
