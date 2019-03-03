use crate::{
    game::GameState,
    na::geometry::Perspective3,
    renderer::{RenderData, Renderer, RendererResult},
    types::{prelude::*, Color},
    utils::clamp,
    vulkan::{
        buffer::Buffer,
        descriptor::{DescriptorSetLayout, DescriptorSetLayoutBinding},
        error::{VulkanError, VulkanResult},
        one_time_command_buffer::OneTimeCommandBuffer,
        text::{GlyphMetrics, TextVertex},
        texture::Texture,
        vertex::{Vertex2f, Vertex3f},
        vertex_input::VertexInput,
        VulkanCore,
    },
};
use ash::{
    prelude::VkResult,
    version::{DeviceV1_0, InstanceV1_0},
    vk,
};
use byteorder::ByteOrder;
use byteorder::LittleEndian;
use image;
use std::{collections::HashMap, ffi::CString, fs::File, io::prelude::*, path::Path};
use winit::{dpi::LogicalSize, EventsLoop, Window};

// Pin to swapchain len for now
const MAX_FRAMES_IN_FLIGHT: usize = 3;
const VERTEX_BUFFER_CAPCITY: vk::DeviceSize = 1 << 20;
const N_GLYPH_TEXTURES: usize = 256;
const CROSSHAIR_WIDTH: f32 = 32.0;
const CROSSHAIR_HEIGHT: f32 = 32.0;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct UiUniforms {
    pub screen_space_normalize_mat: Matrix4f,
}

impl UiUniforms {
    fn new(screen_space_normalize_mat: Matrix4f) -> UiUniforms {
        UiUniforms {
            screen_space_normalize_mat,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct UniformPushConstants {
    pub proj_view: Matrix4f,
}

impl UniformPushConstants {
    fn to_vec(&self) -> Vec<f32> {
        self.proj_view.as_slice().to_vec()
    }
}

impl Default for UniformPushConstants {
    fn default() -> Self {
        UniformPushConstants {
            proj_view: Matrix4f::identity(),
        }
    }
}

unsafe fn has_stencil_component(format: vk::Format) -> bool {
    format == vk::Format::D32_SFLOAT_S8_UINT || format == vk::Format::D24_UNORM_S8_UINT
}

fn pad_font_bytes(bytes: Vec<u8>) -> Vec<u8> {
    let mut v = vec![];
    for b in bytes {
        v.push(b);
        v.push(b);
        v.push(b);
        v.push(b);
    }
    v
}

unsafe fn new_graphics_descriptor_set_layout(
    graphics_texture_sampler: vk::Sampler,
) -> VkResult<DescriptorSetLayout> {
    // graphics
    let sampler_layout_binding = DescriptorSetLayoutBinding::new(
        1,
        vk::DescriptorType::SAMPLER,
        1,
        vk::ShaderStageFlags::FRAGMENT,
        vec![graphics_texture_sampler],
    );
    let texture_layout_binding = DescriptorSetLayoutBinding::new(
        2,
        vk::DescriptorType::SAMPLED_IMAGE,
        1,
        vk::ShaderStageFlags::FRAGMENT,
        vec![],
    );
    Ok(DescriptorSetLayout::new(vec![
        sampler_layout_binding,
        texture_layout_binding,
    ])?)
}

unsafe fn new_text_descriptor_set_layout(
    text_texture_sampler: vk::Sampler,
) -> VkResult<DescriptorSetLayout> {
    let text_uniforms_layout_binding = DescriptorSetLayoutBinding::new(
        0,
        vk::DescriptorType::UNIFORM_BUFFER,
        1,
        vk::ShaderStageFlags::VERTEX,
        vec![],
    );
    let sampler_layout_binding = DescriptorSetLayoutBinding::new(
        1,
        vk::DescriptorType::SAMPLER,
        1,
        vk::ShaderStageFlags::FRAGMENT,
        vec![text_texture_sampler],
    );
    let glyph_textures_binding = DescriptorSetLayoutBinding::new(
        2,
        vk::DescriptorType::SAMPLED_IMAGE,
        N_GLYPH_TEXTURES as u32,
        vk::ShaderStageFlags::FRAGMENT,
        vec![],
    );
    Ok(DescriptorSetLayout::new(vec![
        text_uniforms_layout_binding,
        sampler_layout_binding,
        glyph_textures_binding,
    ])?)
}

unsafe fn new_crosshair_descriptor_set_layout(
    graphics_texture_sampler: vk::Sampler,
) -> VkResult<DescriptorSetLayout> {
    let crosshair_uniforms_layout_binding = DescriptorSetLayoutBinding::new(
        0,
        vk::DescriptorType::UNIFORM_BUFFER,
        1,
        vk::ShaderStageFlags::VERTEX,
        vec![],
    );
    let sampler_layout_binding = DescriptorSetLayoutBinding::new(
        1,
        vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        1,
        vk::ShaderStageFlags::FRAGMENT,
        vec![graphics_texture_sampler],
    );
    Ok(DescriptorSetLayout::new(vec![
        crosshair_uniforms_layout_binding,
        sampler_layout_binding,
    ])?)
}

unsafe fn new_selection_descriptor_set_layout(
    graphics_texture_sampler: vk::Sampler,
) -> VkResult<DescriptorSetLayout> {
    let sampler_layout_binding = DescriptorSetLayoutBinding::new(
        0,
        vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        1,
        vk::ShaderStageFlags::FRAGMENT,
        vec![graphics_texture_sampler],
    );
    Ok(DescriptorSetLayout::new(vec![sampler_layout_binding])?)
}

pub struct VulkanApp {
    pub core: VulkanCore,
    current_frame: usize,

    screen_width: u32,
    screen_height: u32,

    surface_format: vk::SurfaceFormatKHR,
    swapchain_extent: vk::Extent2D,
    swapchain_handle: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    swapchain_framebuffers: Vec<vk::Framebuffer>,
    swapchain_len: usize,

    render_pass: vk::RenderPass,

    glyph_metrics: Vec<GlyphMetrics>,
    glyph_textures: Vec<Texture>,

    transfer_command_pool: vk::CommandPool,
    descriptor_pool: vk::DescriptorPool,
    push_const_cmd_bufs: Vec<Option<vk::CommandBuffer>>,

    // graphics
    graphics_command_pool: vk::CommandPool,

    graphics_pipeline: vk::Pipeline,
    graphics_pipeline_layout: vk::PipelineLayout,

    graphics_staging_vertex_buffers: Vec<Buffer<Vertex3f>>,
    graphics_vertex_buffers: Vec<Buffer<Vertex3f>>,
    graphics_draw_cmd_bufs: Vec<Option<vk::CommandBuffer>>,

    graphics_descriptor_sets: Vec<vk::DescriptorSet>,
    graphics_descriptor_set_layout: DescriptorSetLayout,

    graphics_texture_sampler: vk::Sampler,

    // text
    text_pipeline: vk::Pipeline,
    text_pipeline_layout: vk::PipelineLayout,

    text_staging_vertex_buffers: Vec<Buffer<TextVertex>>,
    text_vertex_buffers: Vec<Buffer<TextVertex>>,
    text_draw_cmd_bufs: Vec<Option<vk::CommandBuffer>>,

    text_uniform_buffers: Vec<Buffer<UiUniforms>>,

    text_descriptor_sets: Vec<vk::DescriptorSet>,
    text_descriptor_set_layout: DescriptorSetLayout,

    text_texture_sampler: vk::Sampler,

    // selection
    selection_pipeline: vk::Pipeline,
    selection_pipeline_layout: vk::PipelineLayout,

    selection_staging_vertex_buffers: Vec<Buffer<Vertex3f>>,
    selection_vertex_buffers: Vec<Buffer<Vertex3f>>,
    selection_draw_cmd_bufs: Vec<Option<vk::CommandBuffer>>,

    selection_descriptor_sets: Vec<vk::DescriptorSet>,
    selection_descriptor_set_layout: DescriptorSetLayout,

    // crosshair
    crosshair_pipeline: vk::Pipeline,
    crosshair_pipeline_layout: vk::PipelineLayout,

    crosshair_staging_vertex_buffer: Buffer<Vertex2f>,
    crosshair_vertex_buffer: Buffer<Vertex2f>,
    crosshair_transfer_cmd_buf: vk::CommandBuffer,
    crosshair_draw_cmd_bufs: Vec<vk::CommandBuffer>,

    crosshair_descriptor_sets: Vec<vk::DescriptorSet>,
    crosshair_descriptor_set_layout: DescriptorSetLayout,

    render_pass_cmd_bufs: Vec<Option<vk::CommandBuffer>>,
    transfer_cmd_bufs: Vec<Option<vk::CommandBuffer>>,
    take_ownership_cmd_buffers: Vec<vk::CommandBuffer>,

    textures: HashMap<String, Texture>,

    depth_image: vk::Image,
    depth_image_memory: vk::DeviceMemory,
    depth_image_view: vk::ImageView,

    msaa_samples: vk::SampleCountFlags,

    color_image: vk::Image,
    color_image_memory: vk::DeviceMemory,
    color_image_view: vk::ImageView,

    view_mat: Matrix4f,
    proj_mat: Matrix4f,
    screen_space_normalize_mat: Matrix4f,
    uniform_push_constants: UniformPushConstants,

    // sync
    transfer_ownership_semaphores: Vec<vk::Semaphore>,
    push_const_semaphores: Vec<vk::Semaphore>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
}

impl VulkanApp {
    pub fn new(screen_width: u32, screen_height: u32) -> VulkanResult<VulkanApp> {
        unsafe {
            let core = VulkanCore::new("Minecrust", screen_width, screen_height)?;

            let view_mat = Matrix4f::look_at_rh(
                &Point3f::new(5.0, 1.0, 5.0),
                &Point3f::origin(),
                &Vector3f::y_axis(),
            );
            let mut flip_mat = Matrix4f::from_diagonal(&Vector4f::new(1.0, -1.0, 0.5, 1.0));
            flip_mat[(2, 3)] = 0.5;
            let proj_mat = flip_mat
                * Perspective3::new(
                    screen_width as f32 / screen_height as f32,
                    f32::to_radians(45.0),
                    0.1,
                    100.0,
                )
                .to_homogeneous();

            let mut screen_space_normalize_mat = Matrix3f::new_nonuniform_scaling(&Vector2f::new(
                1.0 / (screen_width as f32 / 2.0),
                1.0 / (screen_height as f32 / 2.0),
            ));
            screen_space_normalize_mat[(0, 2)] = -1.0;
            screen_space_normalize_mat[(1, 2)] = -1.0;
            let screen_space_normalize_mat = Matrix4f::from_matrix3f(screen_space_normalize_mat);

            let graphics_texture_sampler = core.device.create_sampler(
                &vk::SamplerCreateInfo::builder()
                    .mag_filter(vk::Filter::NEAREST)
                    .min_filter(vk::Filter::LINEAR)
                    .address_mode_u(vk::SamplerAddressMode::REPEAT)
                    .address_mode_v(vk::SamplerAddressMode::REPEAT)
                    .address_mode_w(vk::SamplerAddressMode::REPEAT)
                    .anisotropy_enable(true)
                    .max_anisotropy(16.0)
                    .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
                    .unnormalized_coordinates(false)
                    .compare_enable(false)
                    .compare_op(vk::CompareOp::ALWAYS)
                    .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                    .mip_lod_bias(0.0)
                    .min_lod(0.0)
                    .max_lod(100.0)
                    .build(),
                None,
            )?;
            let text_texture_sampler = core.device.create_sampler(
                &vk::SamplerCreateInfo::builder()
                    .mag_filter(vk::Filter::LINEAR)
                    .min_filter(vk::Filter::LINEAR)
                    .address_mode_u(vk::SamplerAddressMode::REPEAT)
                    .address_mode_v(vk::SamplerAddressMode::REPEAT)
                    .address_mode_w(vk::SamplerAddressMode::REPEAT)
                    .anisotropy_enable(true)
                    .max_anisotropy(16.0)
                    .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
                    .unnormalized_coordinates(false)
                    .compare_enable(false)
                    .compare_op(vk::CompareOp::ALWAYS)
                    .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                    .build(),
                None,
            )?;

            let graphics_descriptor_set_layout =
                new_graphics_descriptor_set_layout(graphics_texture_sampler)?;
            let text_descriptor_set_layout = new_text_descriptor_set_layout(text_texture_sampler)?;
            let crosshair_descriptor_set_layout =
                new_crosshair_descriptor_set_layout(graphics_texture_sampler)?;
            let selection_descriptor_set_layout =
                new_selection_descriptor_set_layout(graphics_texture_sampler)?;

            let crosshair_staging_vertex_buffer = Buffer::new(
                &core,
                (std::mem::size_of::<Vertex2f>() * 6) as vk::DeviceSize,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            let crosshair_vertex_buffer = Buffer::new(
                &core,
                (std::mem::size_of::<Vertex2f>() * 6) as vk::DeviceSize,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            );

            let mut base = VulkanApp {
                core,
                current_frame: 0,

                screen_width,
                screen_height,

                surface_format: Default::default(),
                swapchain_extent: Default::default(),
                swapchain_handle: Default::default(),
                swapchain_images: Default::default(),
                swapchain_image_views: Default::default(),
                swapchain_framebuffers: Default::default(),
                swapchain_len: 0,

                render_pass: Default::default(),

                glyph_metrics: Default::default(),
                glyph_textures: Default::default(),

                transfer_command_pool: Default::default(),
                descriptor_pool: Default::default(),
                push_const_cmd_bufs: Default::default(),

                // graphics
                graphics_command_pool: Default::default(),

                graphics_pipeline: Default::default(),
                graphics_pipeline_layout: Default::default(),

                graphics_staging_vertex_buffers: Default::default(),
                graphics_vertex_buffers: Default::default(),
                graphics_draw_cmd_bufs: Default::default(),

                graphics_descriptor_sets: Default::default(),
                graphics_descriptor_set_layout,

                graphics_texture_sampler,

                // text
                text_pipeline: Default::default(),
                text_pipeline_layout: Default::default(),

                text_staging_vertex_buffers: Default::default(),
                text_vertex_buffers: Default::default(),
                text_draw_cmd_bufs: Default::default(),

                text_uniform_buffers: Default::default(),

                text_descriptor_sets: Default::default(),
                text_descriptor_set_layout,

                text_texture_sampler,

                // selection
                selection_pipeline: Default::default(),
                selection_pipeline_layout: Default::default(),

                selection_staging_vertex_buffers: Default::default(),
                selection_vertex_buffers: Default::default(),
                selection_draw_cmd_bufs: Default::default(),

                selection_descriptor_sets: Default::default(),
                selection_descriptor_set_layout,

                // crosshair
                crosshair_pipeline: Default::default(),
                crosshair_pipeline_layout: Default::default(),

                crosshair_staging_vertex_buffer,
                crosshair_vertex_buffer,
                crosshair_transfer_cmd_buf: Default::default(),
                crosshair_draw_cmd_bufs: Default::default(),

                crosshair_descriptor_sets: Default::default(),
                crosshair_descriptor_set_layout,

                render_pass_cmd_bufs: Default::default(),
                transfer_cmd_bufs: Default::default(),
                take_ownership_cmd_buffers: Default::default(),
                transfer_ownership_semaphores: Default::default(),

                textures: HashMap::default(),

                depth_image: Default::default(),
                depth_image_memory: Default::default(),
                depth_image_view: Default::default(),

                msaa_samples: Default::default(),

                color_image: Default::default(),
                color_image_memory: Default::default(),
                color_image_view: Default::default(),

                view_mat,
                proj_mat,
                screen_space_normalize_mat,
                uniform_push_constants: UniformPushConstants {
                    proj_view: proj_mat * view_mat,
                },

                // sync
                push_const_semaphores: Default::default(),
                image_available_semaphores: Default::default(),
                render_finished_semaphores: Default::default(),
                in_flight_fences: Default::default(),
            };

            base.msaa_samples = base.get_max_usable_sample_count();

            base.create_swapchain(screen_width, screen_height)?;

            base.create_render_pass()?;

            base.init_descriptor_set_layouts()?;
            base.create_graphics_pipeline()?;
            base.create_text_pipeline()?;
            base.create_crosshair_pipeline()?;
            base.create_selection_pipeline()?;

            base.create_command_pools()?;

            base.create_color_resources()?;
            base.create_depth_resources()?;
            base.create_framebuffers()?;

            // fonts
            let ft_lib = freetype::Library::init()?;
            let face = ft_lib.new_face("assets/fonts/SourceCodePro-Regular.ttf", 0)?;
            face.set_pixel_sizes(0, 96)?;

            for c in 0..=255u8 {
                let c = c as char;
                face.load_char(c as usize, freetype::face::LoadFlag::RENDER)?;
                let glyph = face.glyph();
                let bitmap = glyph.bitmap();
                assert_eq!(glyph.metrics().width / 64, bitmap.width());
                let (buffer, dimensions) = if !bitmap.buffer().is_empty() {
                    (
                        bitmap.buffer().to_vec(),
                        (bitmap.width() as u32, bitmap.rows() as u32),
                    )
                } else {
                    (vec![0], (1, 1))
                };
                let texture =
                    base.new_texture_image_from_bytes(&pad_font_bytes(buffer), dimensions, 1)?;

                base.glyph_textures.push(texture);
                base.glyph_metrics.push(glyph.metrics().into());
            }

            base.textures.insert(
                "texture".to_owned(),
                base.create_texture_image("assets/texture.jpg")?,
            );
            base.textures.insert(
                "cobblestone".to_owned(),
                base.create_texture_image("assets/cobblestone-border-arrow.png")?,
            );
            base.textures.insert(
                "crosshair".to_owned(),
                base.create_texture_image("assets/crosshair.png")?,
            );
            base.textures.insert(
                "selection".to_owned(),
                base.create_texture_image("assets/selection.png")?,
            );

            base.crosshair_staging_vertex_buffer.init(&base.core)?;
            base.crosshair_vertex_buffer.init(&base.core)?;

            base.crosshair_staging_vertex_buffer.map()?;
            base.update_crosshair_vtx_buf(screen_width, screen_height)?;

            base.create_buffers()?;

            base.create_descriptor_pool()?;
            base.create_descriptor_sets()?;
            base.create_sync_objects()?;

            for i in 0..base.swapchain_len {
                base.crosshair_draw_cmd_bufs
                    .push(base.new_crosshair_draw_cmd_buf(i)?);
            }

            base.copy_crosshair_vertices()?;

            Ok(base)
        }
    }

    unsafe fn get_max_usable_sample_count(&self) -> vk::SampleCountFlags {
        let physical_device_properties = self
            .core
            .instance
            .get_physical_device_properties(self.core.physical_device);
        let counts = Ord::min(
            physical_device_properties
                .limits
                .framebuffer_color_sample_counts,
            physical_device_properties
                .limits
                .framebuffer_depth_sample_counts,
        );
        if counts.contains(vk::SampleCountFlags::TYPE_64) {
            vk::SampleCountFlags::TYPE_64
        } else if counts.contains(vk::SampleCountFlags::TYPE_32) {
            vk::SampleCountFlags::TYPE_32
        } else if counts.contains(vk::SampleCountFlags::TYPE_16) {
            vk::SampleCountFlags::TYPE_16
        } else if counts.contains(vk::SampleCountFlags::TYPE_8) {
            vk::SampleCountFlags::TYPE_8
        } else if counts.contains(vk::SampleCountFlags::TYPE_4) {
            vk::SampleCountFlags::TYPE_4
        } else if counts.contains(vk::SampleCountFlags::TYPE_2) {
            vk::SampleCountFlags::TYPE_2
        } else {
            vk::SampleCountFlags::TYPE_1
        }
    }

    unsafe fn create_swapchain(
        &mut self,
        screen_width: u32,
        screen_height: u32,
    ) -> VulkanResult<()> {
        let surface_formats = self.core.surface.get_physical_device_surface_formats(
            self.core.physical_device,
            self.core.surface_handle,
        )?;
        self.surface_format = surface_formats
            .iter()
            .map(|sfmt| match sfmt.format {
                vk::Format::UNDEFINED => vk::SurfaceFormatKHR {
                    format: vk::Format::B8G8R8_UNORM,
                    color_space: sfmt.color_space,
                },
                _ => *sfmt,
            })
            .nth(0)
            .expect("Unable to find suitable surface format");

        let surface_capabilities = self.core.surface.get_physical_device_surface_capabilities(
            self.core.physical_device,
            self.core.surface_handle,
        )?;

        self.swapchain_extent =
            Self::choose_swapchain_extent(surface_capabilities, screen_width, screen_height);
        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };

        let mut image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && image_count > surface_capabilities.max_image_count
        {
            image_count = surface_capabilities.max_image_count;
        }
        let present_modes = self
            .core
            .surface
            .get_physical_device_surface_present_modes(
                self.core.physical_device,
                self.core.surface_handle,
            )?;
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::FIFO)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        let swapchain_ci = vk::SwapchainCreateInfoKHR::builder()
            .surface(self.core.surface_handle)
            .min_image_count(image_count)
            .image_color_space(self.surface_format.color_space)
            .image_format(self.surface_format.format)
            .image_extent(self.swapchain_extent)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null())
            .image_array_layers(1)
            .build();
        self.swapchain_handle = self.core.swapchain.create_swapchain(&swapchain_ci, None)?;
        self.swapchain_images = self
            .core
            .swapchain
            .get_swapchain_images(self.swapchain_handle)?;
        self.swapchain_image_views.clear();
        for &swapchain_image in &self.swapchain_images {
            self.swapchain_image_views.push(self.create_image_view(
                swapchain_image,
                self.surface_format.format,
                1,
                vk::ImageAspectFlags::COLOR,
            )?);
        }

        self.swapchain_len = self.swapchain_images.len();

        Ok(())
    }

    fn create_descriptor_pool(&mut self) -> VkResult<()> {
        let mut bindings = HashMap::new();
        let layouts = [
            &self.graphics_descriptor_set_layout,
            &self.text_descriptor_set_layout,
            &self.crosshair_descriptor_set_layout,
            &self.selection_descriptor_set_layout,
        ];
        for layout in &layouts {
            for binding in &layout.bindings {
                *bindings.entry(binding.ty).or_insert(0) += 1;
            }
        }

        let pool_sizes = bindings
            .into_iter()
            .map(|(k, v)| {
                vk::DescriptorPoolSize::builder()
                    .ty(k)
                    .descriptor_count(v * self.swapchain_len as u32)
                    .build()
            })
            .collect::<Vec<_>>();
        let create_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets((self.swapchain_len * layouts.len()) as u32)
            .build();

        unsafe {
            self.descriptor_pool = self
                .core
                .device
                .create_descriptor_pool(&create_info, None)?;
        }

        Ok(())
    }

    fn init_descriptor_set_layouts(&mut self) -> VkResult<()> {
        self.graphics_descriptor_set_layout.init(&self.core)?;
        self.text_descriptor_set_layout.init(&self.core)?;
        self.crosshair_descriptor_set_layout.init(&self.core)?;
        self.selection_descriptor_set_layout.init(&self.core)?;
        Ok(())
    }

    unsafe fn create_render_pass(&mut self) -> VulkanResult<()> {
        let color_attachment_description = vk::AttachmentDescription::builder()
            .format(self.surface_format.format)
            .samples(self.msaa_samples)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();
        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let depth_format = self
            .depth_format()
            .ok_or_else(|| VulkanError::Str("couldn't find depth format".to_owned()))?;
        let depth_attachment_description = vk::AttachmentDescription::builder()
            .format(depth_format)
            .samples(self.msaa_samples)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();
        let depth_attachment_ref = vk::AttachmentReference::builder()
            .attachment(1)
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();

        let color_attachment_resolve = vk::AttachmentDescription::builder()
            .format(self.surface_format.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::DONT_CARE)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build();
        let color_attachment_resolve_ref = vk::AttachmentReference::builder()
            .attachment(2)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let color_attachments = [color_attachment_ref];
        let resolve_attachments = [color_attachment_resolve_ref];
        let subpass_description = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments)
            .depth_stencil_attachment(&depth_attachment_ref)
            .resolve_attachments(&resolve_attachments)
            .build();
        let subpass_dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            )
            .build();
        let attachments = [
            color_attachment_description,
            depth_attachment_description,
            color_attachment_resolve,
        ];
        let subpasses = [subpass_description];
        let dependencies = [subpass_dependency];
        let render_pass_ci = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies)
            .build();
        self.render_pass = self.core.device.create_render_pass(&render_pass_ci, None)?;
        Ok(())
    }

    fn create_pipeline<V: VertexInput>(
        &self,
        vert_module: vk::ShaderModule,
        frag_module: vk::ShaderModule,
        color_blend_attachment: vk::PipelineColorBlendAttachmentState,
        depth_stencil_state: vk::PipelineDepthStencilStateCreateInfo,
        push_constant_range: Option<vk::PushConstantRange>,
        layout: &DescriptorSetLayout,
    ) -> VulkanResult<(vk::PipelineLayout, vk::Pipeline)> {
        let shader_stage_name = CString::new("main")?;
        let vert_shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(shader_stage_name.as_c_str())
            .build();
        let frag_shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(shader_stage_name.as_c_str())
            .build();
        let shader_stages = [vert_shader_stage_create_info, frag_shader_stage_create_info];

        let vertex_binding_descriptions = [V::binding_description()];
        let vertex_attribute_descriptions = V::attribute_descriptions();

        let vertex_input_state_ci = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_binding_descriptions)
            .vertex_attribute_descriptions(&vertex_attribute_descriptions)
            .build();

        let input_assembly_state_ci = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(self.swapchain_extent.width as f32)
            .height(self.swapchain_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build();
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: self.swapchain_extent,
        };
        let viewports = [viewport];
        let scissors = [scissor];
        let viewport_state_ci = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors)
            .build();
        let rasterizer_state_ci = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .build();
        let multisample_state_ci = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(true)
            .rasterization_samples(self.msaa_samples)
            .min_sample_shading(0.2)
            .build();
        let attachments = [color_blend_attachment];
        let color_blend_state_ci = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&attachments)
            .build();
        let layouts = vec![layout.layout(); self.swapchain_len];
        let pipeline_layout_ci = if let Some(push_const_range) = push_constant_range {
            let push_constant_ranges = [push_const_range];
            vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&layouts)
                .push_constant_ranges(&push_constant_ranges)
                .build()
        } else {
            vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&layouts)
                .build()
        };
        let pipeline_layout = unsafe {
            self.core
                .device
                .create_pipeline_layout(&pipeline_layout_ci, None)?
        };

        let pipeline_ci = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_state_ci)
            .input_assembly_state(&input_assembly_state_ci)
            .viewport_state(&viewport_state_ci)
            .rasterization_state(&rasterizer_state_ci)
            .multisample_state(&multisample_state_ci)
            .color_blend_state(&color_blend_state_ci)
            .depth_stencil_state(&depth_stencil_state)
            .layout(pipeline_layout)
            .render_pass(self.render_pass)
            .subpass(0)
            .build();

        let pipeline_cis = [pipeline_ci];
        let pipeline = unsafe {
            self.core
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_cis, None)
                .map_err(|x| x.1)?[0]
        };

        // clean up shader modules
        unsafe {
            self.core.device.destroy_shader_module(vert_module, None);
            self.core.device.destroy_shader_module(frag_module, None);
        }

        Ok((pipeline_layout, pipeline))
    }

    unsafe fn create_graphics_pipeline(&mut self) -> VulkanResult<()> {
        let (pipeline_layout, pipeline) = self.create_pipeline::<Vertex3f>(
            self.create_shader_module_from_file("src/shaders/graphics-vert.spv")?,
            self.create_shader_module_from_file("src/shaders/graphics-frag.spv")?,
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::all())
                .blend_enable(false)
                .build(),
            vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(vk::CompareOp::LESS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .build(),
            Some(
                vk::PushConstantRange::builder()
                    .stage_flags(vk::ShaderStageFlags::VERTEX)
                    .size(std::mem::size_of::<UniformPushConstants>() as u32)
                    .offset(0)
                    .build(),
            ),
            &self.graphics_descriptor_set_layout,
        )?;
        self.graphics_pipeline_layout = pipeline_layout;
        self.graphics_pipeline = pipeline;
        Ok(())
    }

    unsafe fn create_text_pipeline(&mut self) -> VulkanResult<()> {
        let (pipeline_layout, pipeline) = self.create_pipeline::<TextVertex>(
            self.create_shader_module_from_file("src/shaders/text-vert.spv")?,
            self.create_shader_module_from_file("src/shaders/text-frag.spv")?,
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::all())
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .alpha_blend_op(vk::BlendOp::ADD)
                .build(),
            vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(false)
                .depth_write_enable(false)
                .depth_compare_op(vk::CompareOp::LESS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .build(),
            None,
            &self.text_descriptor_set_layout,
        )?;
        self.text_pipeline_layout = pipeline_layout;
        self.text_pipeline = pipeline;
        Ok(())
    }

    unsafe fn create_crosshair_pipeline(&mut self) -> VulkanResult<()> {
        let (pipeline_layout, pipeline) = self.create_pipeline::<Vertex2f>(
            self.create_shader_module_from_file("src/shaders/crosshair-vert.spv")?,
            self.create_shader_module_from_file("src/shaders/crosshair-frag.spv")?,
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::all())
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .alpha_blend_op(vk::BlendOp::ADD)
                .build(),
            vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(false)
                .depth_write_enable(false)
                .depth_compare_op(vk::CompareOp::LESS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .build(),
            None,
            &self.crosshair_descriptor_set_layout,
        )?;
        self.crosshair_pipeline_layout = pipeline_layout;
        self.crosshair_pipeline = pipeline;
        Ok(())
    }

    unsafe fn create_selection_pipeline(&mut self) -> VulkanResult<()> {
        let (pipeline_layout, pipeline) = self.create_pipeline::<Vertex3f>(
            self.create_shader_module_from_file("src/shaders/selection-vert.spv")?,
            self.create_shader_module_from_file("src/shaders/selection-frag.spv")?,
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::all())
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .alpha_blend_op(vk::BlendOp::ADD)
                .build(),
            vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .build(),
            Some(
                vk::PushConstantRange::builder()
                    .stage_flags(vk::ShaderStageFlags::VERTEX)
                    .size(std::mem::size_of::<UniformPushConstants>() as u32)
                    .offset(0)
                    .build(),
            ),
            &self.selection_descriptor_set_layout,
        )?;
        self.selection_pipeline_layout = pipeline_layout;
        self.selection_pipeline = pipeline;
        Ok(())
    }

    unsafe fn create_framebuffers(&mut self) -> VkResult<()> {
        self.swapchain_framebuffers = vec![];
        for &swapchain_image_view in &self.swapchain_image_views {
            let attachments = [
                self.color_image_view,
                self.depth_image_view,
                swapchain_image_view,
            ];
            let framebuffer_ci = vk::FramebufferCreateInfo::builder()
                .render_pass(self.render_pass)
                .attachments(&attachments)
                .width(self.swapchain_extent.width)
                .height(self.swapchain_extent.height)
                .layers(1)
                .build();
            let framebuffer = self.core.device.create_framebuffer(&framebuffer_ci, None)?;
            self.swapchain_framebuffers.push(framebuffer);
        }
        Ok(())
    }

    unsafe fn create_command_pools(&mut self) -> VkResult<()> {
        let graphics_command_pool_ci = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(self.core.graphics_queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .build();
        self.graphics_command_pool = self
            .core
            .device
            .create_command_pool(&graphics_command_pool_ci, None)?;
        let transfer_command_pool_ci = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(self.core.transfer_queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .build();
        self.transfer_command_pool = self
            .core
            .device
            .create_command_pool(&transfer_command_pool_ci, None)?;
        Ok(())
    }

    unsafe fn new_render_pass_command_buffer(
        &mut self,
        index: usize,
        secondary_command_buffers: &[vk::CommandBuffer],
    ) -> VkResult<vk::CommandBuffer> {
        let cmd_buf = if let Some(cmd_buf) = self.render_pass_cmd_bufs[index] {
            cmd_buf
        } else {
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(self.graphics_command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1)
                .build();
            self.core
                .device
                .allocate_command_buffers(&command_buffer_alloc_info)?[0]
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .build();
        self.core
            .device
            .begin_command_buffer(cmd_buf, &begin_info)?;

        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(self.swapchain_framebuffers[index])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain_extent,
            })
            .clear_values(&clear_values)
            .build();

        self.core.device.cmd_begin_render_pass(
            cmd_buf,
            &render_pass_begin_info,
            vk::SubpassContents::SECONDARY_COMMAND_BUFFERS,
        );

        self.core
            .device
            .cmd_execute_commands(cmd_buf, secondary_command_buffers);

        self.core.device.cmd_end_render_pass(cmd_buf);
        self.core.device.end_command_buffer(cmd_buf)?;

        self.render_pass_cmd_bufs[index] = Some(cmd_buf);

        Ok(cmd_buf)
    }

    unsafe fn create_take_ownership_cmd_bufs(&mut self) -> VkResult<()> {
        for index in 0..self.swapchain_len {
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(self.graphics_command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1)
                .build();
            let command_buffer = self
                .core
                .device
                .allocate_command_buffers(&command_buffer_alloc_info)?[0];

            let begin_info = vk::CommandBufferBeginInfo::builder().build();
            self.core
                .device
                .begin_command_buffer(command_buffer, &begin_info)?;

            self.core.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::VERTEX_INPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[
                    vk::BufferMemoryBarrier::builder()
                        .src_access_mask(vk::AccessFlags::empty())
                        .dst_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ)
                        .src_queue_family_index(self.core.transfer_queue_family_index)
                        .dst_queue_family_index(self.core.graphics_queue_family_index)
                        .buffer(self.graphics_vertex_buffers[index].buffer())
                        .build(),
                    vk::BufferMemoryBarrier::builder()
                        .src_access_mask(vk::AccessFlags::empty())
                        .dst_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ)
                        .src_queue_family_index(self.core.transfer_queue_family_index)
                        .dst_queue_family_index(self.core.graphics_queue_family_index)
                        .buffer(self.text_vertex_buffers[index].buffer())
                        .build(),
                ],
                &[],
            );

            self.core.device.end_command_buffer(command_buffer)?;

            self.take_ownership_cmd_buffers.push(command_buffer);
        }

        Ok(())
    }

    unsafe fn new_text_draw_cmd_buf(&mut self, index: usize) -> VkResult<vk::CommandBuffer> {
        let cmd_buf = if let Some(cmd_buf) = self.text_draw_cmd_bufs[index] {
            cmd_buf
        } else {
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(self.graphics_command_pool)
                .level(vk::CommandBufferLevel::SECONDARY)
                .command_buffer_count(1)
                .build();
            self.core
                .device
                .allocate_command_buffers(&command_buffer_alloc_info)?[0]
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .inheritance_info(
                &vk::CommandBufferInheritanceInfo::builder()
                    .render_pass(self.render_pass)
                    .subpass(0)
                    .build(),
            )
            .flags(
                vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE
                    | vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            )
            .build();
        self.core
            .device
            .begin_command_buffer(cmd_buf, &begin_info)?;

        self.core.device.cmd_bind_pipeline(
            cmd_buf,
            vk::PipelineBindPoint::GRAPHICS,
            self.text_pipeline,
        );
        self.core.device.cmd_bind_vertex_buffers(
            cmd_buf,
            0,
            &[self.text_vertex_buffers[index].buffer()],
            &[0],
        );
        self.core.device.cmd_bind_descriptor_sets(
            cmd_buf,
            vk::PipelineBindPoint::GRAPHICS,
            self.text_pipeline_layout,
            0,
            &[self.text_descriptor_sets[index]],
            &[],
        );

        self.core.device.cmd_draw(
            cmd_buf,
            self.text_staging_vertex_buffers[index].len as u32,
            1,
            0,
            0,
        );

        self.core.device.end_command_buffer(cmd_buf)?;

        self.text_draw_cmd_bufs[index] = Some(cmd_buf);

        Ok(cmd_buf)
    }

    unsafe fn new_transfer_cmd_buf(
        &mut self,
        index: usize,
        selection_active: bool,
    ) -> VkResult<vk::CommandBuffer> {
        let cmd_buf = if let Some(cmd_buf) = self.transfer_cmd_bufs[index] {
            cmd_buf
        } else {
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(self.transfer_command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1)
                .build();
            self.core
                .device
                .allocate_command_buffers(&command_buffer_alloc_info)?[0]
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .build();
        self.core
            .device
            .begin_command_buffer(cmd_buf, &begin_info)?;

        self.core.device.cmd_copy_buffer(
            cmd_buf,
            self.graphics_staging_vertex_buffers[index].buffer(),
            self.graphics_vertex_buffers[index].buffer(),
            &[vk::BufferCopy::builder()
                .size(self.graphics_staging_vertex_buffers[index].buf_len)
                .build()],
        );

        self.core.device.cmd_copy_buffer(
            cmd_buf,
            self.text_staging_vertex_buffers[index].buffer(),
            self.text_vertex_buffers[index].buffer(),
            &[vk::BufferCopy::builder()
                .size(self.text_staging_vertex_buffers[index].buf_len)
                .build()],
        );

        if selection_active {
            self.core.device.cmd_copy_buffer(
                cmd_buf,
                self.selection_staging_vertex_buffers[index].buffer(),
                self.selection_vertex_buffers[index].buffer(),
                &[vk::BufferCopy::builder()
                    .size(self.selection_staging_vertex_buffers[index].buf_len)
                    .build()],
            );
        }

        self.core.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[
                vk::BufferMemoryBarrier::builder()
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::empty())
                    .src_queue_family_index(self.core.transfer_queue_family_index)
                    .dst_queue_family_index(self.core.graphics_queue_family_index)
                    .buffer(self.graphics_vertex_buffers[index].buffer())
                    .build(),
                vk::BufferMemoryBarrier::builder()
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::empty())
                    .src_queue_family_index(self.core.transfer_queue_family_index)
                    .dst_queue_family_index(self.core.graphics_queue_family_index)
                    .buffer(self.text_vertex_buffers[index].buffer())
                    .build(),
            ],
            &[],
        );

        self.core.device.end_command_buffer(cmd_buf)?;

        self.transfer_cmd_bufs[index] = Some(cmd_buf);

        Ok(cmd_buf)
    }

    fn update_crosshair_vtx_buf(&mut self, screen_width: u32, screen_height: u32) -> VkResult<()> {
        let center_x = screen_width as f32 / 2.0;
        let center_y = screen_height as f32 / 2.0;
        let left_top = Vertex2f::new(
            Point2f::new(
                center_x - CROSSHAIR_WIDTH / 2.0,
                center_y - CROSSHAIR_HEIGHT / 2.0,
            ),
            Point2f::new(0.0, 0.0),
        );
        let left_bottom = Vertex2f::new(
            Point2f::new(
                center_x - CROSSHAIR_WIDTH / 2.0,
                center_y + CROSSHAIR_HEIGHT / 2.0,
            ),
            Point2f::new(0.0, 1.0),
        );
        let right_bottom = Vertex2f::new(
            Point2f::new(
                center_x + CROSSHAIR_WIDTH / 2.0,
                center_y + CROSSHAIR_HEIGHT / 2.0,
            ),
            Point2f::new(1.0, 1.0),
        );
        let right_top = Vertex2f::new(
            Point2f::new(
                center_x + CROSSHAIR_WIDTH / 2.0,
                center_y - CROSSHAIR_HEIGHT / 2.0,
            ),
            Point2f::new(1.0, 0.0),
        );

        self.crosshair_staging_vertex_buffer.copy_data(&[
            left_top,
            left_bottom,
            right_bottom,
            left_top,
            right_bottom,
            right_top,
        ])
    }

    fn new_crosshair_transfer_cmd_buf(&mut self) -> VkResult<vk::CommandBuffer> {
        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.transfer_command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1)
            .build();
        unsafe {
            let cmd_buf = self
                .core
                .device
                .allocate_command_buffers(&command_buffer_alloc_info)?[0];

            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::empty())
                .build();

            self.core
                .device
                .begin_command_buffer(cmd_buf, &begin_info)?;
            self.core.device.cmd_copy_buffer(
                cmd_buf,
                self.crosshair_staging_vertex_buffer.buffer(),
                self.crosshair_vertex_buffer.buffer(),
                &[vk::BufferCopy::builder()
                    .size(self.crosshair_staging_vertex_buffer.buf_len)
                    .build()],
            );
            self.core.device.end_command_buffer(cmd_buf)?;
            Ok(cmd_buf)
        }
    }

    fn copy_crosshair_vertices(&self) -> VkResult<()> {
        let fence = unsafe {
            self.core
                .device
                .create_fence(&vk::FenceCreateInfo::builder().build(), None)?
        };
        let cmd_bufs = [self.crosshair_transfer_cmd_buf];
        let submit_info = vk::SubmitInfo::builder().command_buffers(&cmd_bufs).build();

        unsafe {
            self.core
                .device
                .queue_submit(self.core.transfer_queue, &[submit_info], fence)?;
            self.core
                .device
                .wait_for_fences(&[fence], true, std::u64::MAX)?;
            self.core.device.destroy_fence(fence, None);
        }

        Ok(())
    }

    unsafe fn new_graphics_draw_cmd_buf(&mut self, index: usize) -> VkResult<vk::CommandBuffer> {
        let cmd_buf = if let Some(cmd_buf) = self.graphics_draw_cmd_bufs[index] {
            cmd_buf
        } else {
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(self.graphics_command_pool)
                .level(vk::CommandBufferLevel::SECONDARY)
                .command_buffer_count(1)
                .build();
            self.core
                .device
                .allocate_command_buffers(&command_buffer_alloc_info)?[0]
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .inheritance_info(
                &vk::CommandBufferInheritanceInfo::builder()
                    .render_pass(self.render_pass)
                    .subpass(0)
                    .build(),
            )
            .flags(
                vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE
                    | vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            )
            .build();
        self.core
            .device
            .begin_command_buffer(cmd_buf, &begin_info)?;

        self.core.device.cmd_bind_pipeline(
            cmd_buf,
            vk::PipelineBindPoint::GRAPHICS,
            self.graphics_pipeline,
        );
        self.core.device.cmd_bind_vertex_buffers(
            cmd_buf,
            0,
            &[self.graphics_vertex_buffers[index].buffer()],
            &[0],
        );
        self.core.device.cmd_bind_descriptor_sets(
            cmd_buf,
            vk::PipelineBindPoint::GRAPHICS,
            self.graphics_pipeline_layout,
            0,
            &[self.graphics_descriptor_sets[index]],
            &[],
        );

        self.core.device.cmd_draw(
            cmd_buf,
            self.graphics_staging_vertex_buffers[index].len as u32,
            1,
            0,
            0,
        );

        self.core.device.end_command_buffer(cmd_buf)?;

        self.graphics_draw_cmd_bufs[index] = Some(cmd_buf);

        Ok(cmd_buf)
    }

    fn new_selection_draw_cmd_buf(&mut self, index: usize) -> VkResult<vk::CommandBuffer> {
        let cmd_buf = if let Some(cmd_buf) = self.selection_draw_cmd_bufs[index] {
            cmd_buf
        } else {
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(self.graphics_command_pool)
                .level(vk::CommandBufferLevel::SECONDARY)
                .command_buffer_count(1)
                .build();
            unsafe {
                self.core
                    .device
                    .allocate_command_buffers(&command_buffer_alloc_info)?[0]
            }
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .inheritance_info(
                &vk::CommandBufferInheritanceInfo::builder()
                    .render_pass(self.render_pass)
                    .subpass(0)
                    .build(),
            )
            .flags(
                vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE
                    | vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            )
            .build();

        unsafe {
            self.core
                .device
                .begin_command_buffer(cmd_buf, &begin_info)?;

            self.core.device.cmd_bind_pipeline(
                cmd_buf,
                vk::PipelineBindPoint::GRAPHICS,
                self.selection_pipeline,
            );
            self.core.device.cmd_bind_vertex_buffers(
                cmd_buf,
                0,
                &[self.selection_vertex_buffers[index].buffer()],
                &[0],
            );
            self.core.device.cmd_bind_descriptor_sets(
                cmd_buf,
                vk::PipelineBindPoint::GRAPHICS,
                self.selection_pipeline_layout,
                0,
                &[self.selection_descriptor_sets[index]],
                &[],
            );

            self.core.device.cmd_draw(
                cmd_buf,
                self.selection_staging_vertex_buffers[index].len as u32,
                1,
                0,
                0,
            );

            self.core.device.end_command_buffer(cmd_buf)?;
        }

        self.selection_draw_cmd_bufs[index] = Some(cmd_buf);

        Ok(cmd_buf)
    }

    fn new_crosshair_draw_cmd_buf(&self, index: usize) -> VkResult<vk::CommandBuffer> {
        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.graphics_command_pool)
            .level(vk::CommandBufferLevel::SECONDARY)
            .command_buffer_count(1)
            .build();
        let cmd_buf = unsafe {
            self.core
                .device
                .allocate_command_buffers(&command_buffer_alloc_info)?[0]
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .inheritance_info(
                &vk::CommandBufferInheritanceInfo::builder()
                    .render_pass(self.render_pass)
                    .subpass(0)
                    .build(),
            )
            .flags(vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE)
            .build();

        unsafe {
            self.core
                .device
                .begin_command_buffer(cmd_buf, &begin_info)?;

            self.core.device.cmd_bind_pipeline(
                cmd_buf,
                vk::PipelineBindPoint::GRAPHICS,
                self.crosshair_pipeline,
            );
            self.core.device.cmd_bind_vertex_buffers(
                cmd_buf,
                0,
                &[self.crosshair_vertex_buffer.buffer()],
                &[0],
            );
            self.core.device.cmd_bind_descriptor_sets(
                cmd_buf,
                vk::PipelineBindPoint::GRAPHICS,
                self.crosshair_pipeline_layout,
                0,
                &[self.crosshair_descriptor_sets[index]],
                &[],
            );

            self.core.device.cmd_draw(
                cmd_buf,
                self.crosshair_staging_vertex_buffer.len as u32,
                1,
                0,
                0,
            );

            self.core.device.end_command_buffer(cmd_buf)?;
        }

        Ok(cmd_buf)
    }

    fn new_push_const_cmd_buf(&mut self, index: usize) -> VkResult<vk::CommandBuffer> {
        let cmd_buf = if let Some(cmd_buf) = self.push_const_cmd_bufs[index] {
            cmd_buf
        } else {
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(self.graphics_command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1)
                .build();
            unsafe {
                self.core
                    .device
                    .allocate_command_buffers(&command_buffer_alloc_info)?[0]
            }
        };

        let mut data = [0u8; std::mem::size_of::<UniformPushConstants>()];
        LittleEndian::write_f32_into(&self.uniform_push_constants.to_vec(), &mut data);

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .build();

        unsafe {
            self.core
                .device
                .begin_command_buffer(cmd_buf, &begin_info)?;
            self.core.device.cmd_push_constants(
                cmd_buf,
                self.graphics_pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                &data,
            );
            self.core.device.end_command_buffer(cmd_buf)?;
        }

        self.push_const_cmd_bufs[index] = Some(cmd_buf);
        Ok(cmd_buf)
    }

    unsafe fn create_buffers(&mut self) -> VkResult<()> {
        self.render_pass_cmd_bufs = vec![Default::default(); self.swapchain_len];
        self.push_const_cmd_bufs = vec![Default::default(); self.swapchain_len];

        for _ in 0..self.swapchain_len {
            // graphics
            let mut staging_buf = Buffer::new_init(
                &self.core,
                VERTEX_BUFFER_CAPCITY,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            staging_buf.map()?;
            self.graphics_staging_vertex_buffers.push(staging_buf);

            self.graphics_vertex_buffers.push(Buffer::new_init(
                &self.core,
                VERTEX_BUFFER_CAPCITY,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?);

            // text
            let mut staging_buf = Buffer::new_init(
                &self.core,
                VERTEX_BUFFER_CAPCITY,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            staging_buf.map()?;
            self.text_staging_vertex_buffers.push(staging_buf);

            self.text_vertex_buffers.push(Buffer::new_init(
                &self.core,
                VERTEX_BUFFER_CAPCITY,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?);

            // selection
            let mut staging_buf = Buffer::new_init(
                &self.core,
                VERTEX_BUFFER_CAPCITY,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            staging_buf.map()?;
            self.selection_staging_vertex_buffers.push(staging_buf);

            self.selection_vertex_buffers.push(Buffer::new_init(
                &self.core,
                VERTEX_BUFFER_CAPCITY,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?);

            let mut uniform_buf = Buffer::new_init(
                &self.core,
                std::mem::size_of::<UiUniforms>() as vk::DeviceSize,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            uniform_buf.map()?;
            self.text_uniform_buffers.push(uniform_buf);
        }

        self.graphics_draw_cmd_bufs = vec![Default::default(); self.swapchain_len];
        self.text_draw_cmd_bufs = vec![Default::default(); self.swapchain_len];
        self.selection_draw_cmd_bufs = vec![Default::default(); self.swapchain_len];

        self.crosshair_transfer_cmd_buf = self.new_crosshair_transfer_cmd_buf()?;

        self.transfer_cmd_bufs = vec![Default::default(); self.swapchain_len];
        self.create_take_ownership_cmd_bufs()?;

        Ok(())
    }

    unsafe fn draw_text(
        &mut self,
        index: usize,
        string: &str,
        mut pos: Point2f,
        scale: f32,
        color: Color,
    ) -> VkResult<()> {
        assert!(!string.is_empty());
        assert!(!string.contains('\n'));

        let mut vertices: Vec<TextVertex> = vec![];
        let scale = scale * 0.5;

        let max_bearing_y = string
            .chars()
            .map(|c| self.glyph_metrics[c as usize].bearing_y)
            .max_by(|x, y| x.partial_cmp(y).unwrap())
            .expect("no max_bearing_y") as f32
            * scale;
        let baseline_y = pos.y + max_bearing_y;

        for ch in string.chars() {
            let metrics = &self.glyph_metrics[ch as usize];
            let left_top = TextVertex::new(
                ch as usize,
                Point2f::new(
                    pos.x + metrics.bearing_x * scale,
                    baseline_y - metrics.bearing_y * scale,
                ),
                Point2f::new(0.0, 0.0),
                color,
            );
            let left_bottom = TextVertex::new(
                ch as usize,
                Point2f::new(
                    pos.x + metrics.bearing_x * scale,
                    baseline_y + (metrics.height - metrics.bearing_y) * scale,
                ),
                Point2f::new(0.0, 1.0),
                color,
            );
            let right_bottom = TextVertex::new(
                ch as usize,
                Point2f::new(
                    pos.x + (metrics.bearing_x + metrics.width) * scale,
                    baseline_y + (metrics.height - metrics.bearing_y) * scale,
                ),
                Point2f::new(1.0, 1.0),
                color,
            );
            let right_top = TextVertex::new(
                ch as usize,
                Point2f::new(
                    pos.x + (metrics.bearing_x + metrics.width) * scale,
                    baseline_y - metrics.bearing_y * scale,
                ),
                Point2f::new(1.0, 0.0),
                color,
            );

            vertices.extend(&[
                left_top,
                left_bottom,
                right_bottom,
                left_top,
                right_bottom,
                right_top,
            ]);
            pos.x += metrics.advance * scale;
        }

        let vertex_buffer = &mut self.text_staging_vertex_buffers[index];
        vertex_buffer.copy_data(&vertices)?;
        Ok(())
    }

    pub fn recreate_swapchain(&mut self) -> VulkanResult<()> {
        unsafe {
            self.core.device.device_wait_idle()?;
            self.clean_up_swapchain();
            let LogicalSize {
                width: screen_width,
                height: screen_height,
            } = self.core.window.get_inner_size().unwrap();
            self.screen_width = screen_width as u32;
            self.screen_height = screen_height as u32;
            self.create_swapchain(self.screen_width, self.screen_height)?;

            self.create_render_pass()?;
            self.create_graphics_pipeline()?;
            self.create_text_pipeline()?;
            self.create_crosshair_pipeline()?;
            self.create_selection_pipeline()?;
            self.create_color_resources()?;
            self.create_depth_resources()?;
            self.create_framebuffers()?;

            let mut flip_mat = Matrix4f::from_diagonal(&Vector4f::new(1.0, -1.0, 0.5, 1.0));
            flip_mat[(2, 3)] = 0.5;
            self.proj_mat = flip_mat
                * Perspective3::new(
                    screen_width as f32 / screen_height as f32,
                    f32::to_radians(45.0),
                    0.1,
                    100.0,
                )
                .to_homogeneous();

            let mut screen_space_normalize_mat = Matrix3f::new_nonuniform_scaling(&Vector2f::new(
                1.0 / (screen_width as f32 / 2.0),
                1.0 / (screen_height as f32 / 2.0),
            ));
            screen_space_normalize_mat[(0, 2)] = -1.0;
            screen_space_normalize_mat[(1, 2)] = -1.0;

            self.screen_space_normalize_mat = Matrix4f::from_matrix3f(screen_space_normalize_mat);

            self.uniform_push_constants = UniformPushConstants {
                proj_view: self.proj_mat * self.view_mat,
            };

            for i in 0..self.swapchain_len {
                self.crosshair_draw_cmd_bufs[i] = self.new_crosshair_draw_cmd_buf(i)?;
            }
            self.update_crosshair_vtx_buf(self.screen_width, self.screen_height)?;
            self.copy_crosshair_vertices()?;

            Ok(())
        }
    }

    unsafe fn create_image(
        &self,
        width: u32,
        height: u32,
        mip_levels: u32,
        num_samples: vk::SampleCountFlags,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        memory_properties: vk::MemoryPropertyFlags,
    ) -> VkResult<(vk::Image, vk::DeviceMemory)> {
        let image_ci = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(mip_levels)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(num_samples)
            .build();
        let image = self.core.device.create_image(&image_ci, None)?;
        let memory_requirements = self.core.device.get_image_memory_requirements(image);
        let memory_alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(
                self.find_memory_type(memory_requirements.memory_type_bits, memory_properties)
                    .unwrap(),
            )
            .build();
        let image_memory = self.core.device.allocate_memory(&memory_alloc_info, None)?;
        self.core.device.bind_image_memory(image, image_memory, 0)?;
        Ok((image, image_memory))
    }

    unsafe fn new_texture_image_from_bytes(
        &self,
        bytes: &[u8],
        (img_width, img_height): (u32, u32),
        mip_levels: u32,
    ) -> VulkanResult<Texture> {
        let img_size = vk::DeviceSize::from(img_width * img_height * 4);
        let (staging_buffer, staging_buffer_memory) = self.core.create_buffer(
            img_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        let data = self.core.device.map_memory(
            staging_buffer_memory,
            0,
            img_size,
            vk::MemoryMapFlags::empty(),
        )? as *mut u8;

        assert_eq!(bytes.len(), img_size as usize);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
        self.core.device.unmap_memory(staging_buffer_memory);

        let (texture_image, texture_image_memory) = self.create_image(
            img_width,
            img_height,
            mip_levels,
            vk::SampleCountFlags::TYPE_1,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let command_buffer = OneTimeCommandBuffer::new(
            &self.core.device,
            self.core.graphics_queue,
            self.graphics_command_pool,
        )?;
        self.transition_image_layout(
            &command_buffer,
            texture_image,
            vk::Format::R8G8B8A8_UNORM,
            mip_levels,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )?;
        self.copy_buffer_to_image(
            &command_buffer,
            staging_buffer,
            texture_image,
            img_width,
            img_height,
        )?;
        command_buffer.submit(&[])?;

        self.generate_mipmaps(
            texture_image,
            vk::Format::R8G8B8A8_UNORM,
            img_width,
            img_height,
            mip_levels,
        )?;

        self.core.device.destroy_buffer(staging_buffer, None);
        self.core.device.free_memory(staging_buffer_memory, None);

        let texture_image_view = self.create_texture_image_view(texture_image, mip_levels)?;

        Ok(Texture::new(
            &self.core,
            texture_image,
            texture_image_memory,
            texture_image_view,
        ))
    }

    unsafe fn create_texture_image<P: AsRef<Path>>(&self, path: P) -> VulkanResult<Texture> {
        let img = image::open(path)?.to_rgba();
        let (img_width, img_height) = img.dimensions();
        let mip_levels = f32::floor(f32::log2(u32::max(img_width, img_height) as f32)) as u32 + 1;

        self.new_texture_image_from_bytes(&img.to_vec(), (img_width, img_height), mip_levels)
    }

    unsafe fn create_image_view(
        &self,
        image: vk::Image,
        format: vk::Format,
        mip_levels: u32,
        aspect_flags: vk::ImageAspectFlags,
    ) -> VulkanResult<vk::ImageView> {
        let image_view_ci = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(aspect_flags)
                    .base_mip_level(0)
                    .level_count(mip_levels)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build(),
            )
            .build();
        Ok(self.core.device.create_image_view(&image_view_ci, None)?)
    }

    unsafe fn generate_mipmaps(
        &self,
        image: vk::Image,
        image_format: vk::Format,
        tex_width: u32,
        tex_height: u32,
        mip_levels: u32,
    ) -> VulkanResult<()> {
        let format_properties = self
            .core
            .instance
            .get_physical_device_format_properties(self.core.physical_device, image_format);
        if !format_properties
            .optimal_tiling_features
            .contains(vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR)
        {
            return Err(VulkanError::Str(format!(
                "format {} doesn't support linear blitting",
                image_format,
            )));
        }

        let command_buffer = OneTimeCommandBuffer::new(
            &self.core.device,
            self.core.graphics_queue,
            self.graphics_command_pool,
        )?;

        let mut barrier = vk::ImageMemoryBarrier::builder()
            .image(image)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_array_layer(0)
                    .layer_count(1)
                    .level_count(1)
                    .build(),
            )
            .build();
        let mut mip_width = tex_width as i32;
        let mut mip_height = tex_height as i32;
        for i in 1..mip_levels {
            barrier.subresource_range.base_mip_level = i - 1;
            barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;

            self.core.device.cmd_pipeline_barrier(
                command_buffer.command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            let blit = vk::ImageBlit::builder()
                .src_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: mip_width,
                        y: mip_height,
                        z: 1,
                    },
                ])
                .src_subresource(
                    vk::ImageSubresourceLayers::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .mip_level(i - 1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .dst_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: if mip_width > 1 { mip_width / 2 } else { 1 },
                        y: if mip_height > 1 { mip_height / 2 } else { 1 },
                        z: 1,
                    },
                ])
                .dst_subresource(
                    vk::ImageSubresourceLayers::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .mip_level(i)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .build();
            self.core.device.cmd_blit_image(
                command_buffer.command_buffer,
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[blit],
                vk::Filter::LINEAR,
            );

            barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

            self.core.device.cmd_pipeline_barrier(
                command_buffer.command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
            if mip_width > 1 {
                mip_width /= 2;
            }
            if mip_height > 1 {
                mip_height /= 2;
            }
        }

        barrier.subresource_range.base_mip_level = mip_levels - 1;
        barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

        self.core.device.cmd_pipeline_barrier(
            command_buffer.command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );

        command_buffer.submit(&[])
    }

    unsafe fn create_texture_image_view(
        &self,
        texture_image: vk::Image,
        mip_levels: u32,
    ) -> VulkanResult<vk::ImageView> {
        self.create_image_view(
            texture_image,
            vk::Format::R8G8B8A8_UNORM,
            mip_levels,
            vk::ImageAspectFlags::COLOR,
        )
    }

    unsafe fn transition_image_layout(
        &self,
        command_buffer: &OneTimeCommandBuffer,
        image: vk::Image,
        format: vk::Format,
        mip_levels: u32,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> VulkanResult<()> {
        let mut barrier_builder = vk::ImageMemoryBarrier::builder()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image);

        let aspect_mask = if new_layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            let mut am = vk::ImageAspectFlags::DEPTH;
            if has_stencil_component(format) {
                am |= vk::ImageAspectFlags::STENCIL;
            }
            am
        } else {
            vk::ImageAspectFlags::COLOR
        };

        barrier_builder = barrier_builder.subresource_range(
            vk::ImageSubresourceRange::builder()
                .aspect_mask(aspect_mask)
                .base_mip_level(0)
                .level_count(mip_levels)
                .base_array_layer(0)
                .layer_count(1)
                .build(),
        );

        let source_stage: vk::PipelineStageFlags;
        let destination_stage: vk::PipelineStageFlags;
        if old_layout == vk::ImageLayout::UNDEFINED
            && new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
        {
            barrier_builder = barrier_builder
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);
            source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = vk::PipelineStageFlags::TRANSFER;
        } else if old_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
            && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        {
            barrier_builder = barrier_builder
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ);
            source_stage = vk::PipelineStageFlags::TRANSFER;
            destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
        } else if old_layout == vk::ImageLayout::UNDEFINED
            && new_layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
        {
            barrier_builder = barrier_builder
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(
                    vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                        | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                );
            source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS;
        } else if old_layout == vk::ImageLayout::UNDEFINED
            && new_layout == vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        {
            barrier_builder = barrier_builder
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(
                    vk::AccessFlags::COLOR_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                );
            source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
        } else {
            return Err(VulkanError::Str("unsupported layout transition".to_owned()));
        }

        self.core.device.cmd_pipeline_barrier(
            command_buffer.command_buffer,
            source_stage,
            destination_stage,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier_builder.build()],
        );

        Ok(())
    }

    unsafe fn copy_buffer_to_image(
        &self,
        command_buffer: &OneTimeCommandBuffer,
        buffer: vk::Buffer,
        image: vk::Image,
        width: u32,
        height: u32,
    ) -> VulkanResult<()> {
        self.core.device.cmd_copy_buffer_to_image(
            command_buffer.command_buffer,
            buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[vk::BufferImageCopy::builder()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(
                    vk::ImageSubresourceLayers::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .mip_level(0)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .image_offset(vk::Offset3D::default())
                .image_extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                })
                .build()],
        );

        Ok(())
    }

    unsafe fn create_sync_objects(&mut self) -> VkResult<()> {
        let semaphore_ci = vk::SemaphoreCreateInfo::builder().build();
        let fence_ci = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED)
            .build();

        self.image_available_semaphores = vec![];
        self.render_finished_semaphores = vec![];
        self.push_const_semaphores = vec![];
        self.in_flight_fences = vec![];

        for _ in 0..self.swapchain_len {
            self.transfer_ownership_semaphores
                .push(self.core.device.create_semaphore(&semaphore_ci, None)?);
            self.push_const_semaphores
                .push(self.core.device.create_semaphore(&semaphore_ci, None)?);
        }

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            self.image_available_semaphores
                .push(self.core.device.create_semaphore(&semaphore_ci, None)?);
            self.render_finished_semaphores
                .push(self.core.device.create_semaphore(&semaphore_ci, None)?);
            self.in_flight_fences
                .push(self.core.device.create_fence(&fence_ci, None)?);
        }
        Ok(())
    }

    unsafe fn create_shader_module_from_file<P: AsRef<Path>>(
        &self,
        file: P,
    ) -> VulkanResult<vk::ShaderModule> {
        let mut shader_spv_file = File::open(file)?;
        let mut buf = vec![];
        shader_spv_file.read_to_end(&mut buf)?;
        self.create_shader_module(&buf)
    }

    unsafe fn create_shader_module(&self, code: &[u8]) -> VulkanResult<vk::ShaderModule> {
        let mut code_u32: Vec<u32> = vec![0; code.len() / 4];
        LittleEndian::read_u32_into(code, &mut code_u32);
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(&code_u32)
            .build();
        Ok(self.core.device.create_shader_module(&create_info, None)?)
    }

    unsafe fn choose_swapchain_extent(
        surface_capabilities: vk::SurfaceCapabilitiesKHR,
        screen_width: u32,
        screen_height: u32,
    ) -> vk::Extent2D {
        if surface_capabilities.current_extent.width != std::u32::MAX {
            surface_capabilities.current_extent
        } else {
            let width = clamp(
                surface_capabilities.min_image_extent.width,
                surface_capabilities.max_image_extent.width,
                screen_width,
            );
            let height = clamp(
                surface_capabilities.min_image_extent.height,
                surface_capabilities.max_image_extent.height,
                screen_height,
            );
            vk::Extent2D { width, height }
        }
    }

    unsafe fn find_memory_type(
        &self,
        type_filter: u32,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        let memory_properties = self
            .core
            .instance
            .get_physical_device_memory_properties(self.core.physical_device);
        for i in 0..memory_properties.memory_type_count {
            if type_filter & (1 << i) != 0
                && memory_properties.memory_types[i as usize]
                    .property_flags
                    .contains(memory_property_flags)
            {
                return Some(i);
            }
        }
        None
    }

    unsafe fn update_text_uniform_buffer(&mut self, index: usize) -> VkResult<()> {
        let uniform_buf = &mut self.text_uniform_buffers[index];
        uniform_buf.copy_data(&[UiUniforms::new(self.screen_space_normalize_mat)])?;
        Ok(())
    }

    unsafe fn create_descriptor_sets(&mut self) -> VkResult<()> {
        let layouts = vec![self.graphics_descriptor_set_layout.layout(); self.swapchain_len];
        let descriptor_set_alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&layouts)
            .build();

        self.graphics_descriptor_sets = self
            .core
            .device
            .allocate_descriptor_sets(&descriptor_set_alloc_info)?;
        for &ds in self.graphics_descriptor_sets.iter() {
            // binding 1: sampler
            let sampler_descriptor_image_info = vk::DescriptorImageInfo::builder()
                .sampler(self.graphics_texture_sampler)
                .build();
            let image_info = [sampler_descriptor_image_info];
            let sampler_write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(ds)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(&image_info)
                .build();

            // binding 2: image
            let cobblestone_texture = &self.textures["cobblestone"];
            let texture_descriptor_image_info = vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(cobblestone_texture.view)
                .build();
            let image_info = [texture_descriptor_image_info];
            let texture_write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(ds)
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(&image_info)
                .build();

            self.core.device.update_descriptor_sets(
                &[sampler_write_descriptor_set, texture_write_descriptor_set],
                &[],
            );
        }

        let layouts = vec![self.text_descriptor_set_layout.layout(); self.swapchain_len];
        self.text_descriptor_sets = self.core.device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(self.descriptor_pool)
                .set_layouts(&layouts)
                .build(),
        )?;
        for (i, &ds) in self.text_descriptor_sets.iter().enumerate() {
            let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.text_uniform_buffers[i].buffer())
                .offset(0)
                .range(std::mem::size_of::<UiUniforms>() as vk::DeviceSize)
                .build();
            let buffer_info = [descriptor_buffer_info];
            let buffer_write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(ds)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_info)
                .build();

            let sampler_descriptor_image_info = vk::DescriptorImageInfo::builder()
                .sampler(self.text_texture_sampler)
                .build();
            let image_info = [sampler_descriptor_image_info];
            let sampler_write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(ds)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(&image_info)
                .build();

            let mut image_infos = [Default::default(); N_GLYPH_TEXTURES];
            for (i, image_info) in image_infos.iter_mut().enumerate() {
                *image_info = vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(self.glyph_textures[i].view)
                    .build();
            }

            let glyphs_write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(ds)
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(&image_infos)
                .build();

            self.core.device.update_descriptor_sets(
                &[
                    buffer_write_descriptor_set,
                    sampler_write_descriptor_set,
                    glyphs_write_descriptor_set,
                ],
                &[],
            );
        }

        let layouts = vec![self.crosshair_descriptor_set_layout.layout(); self.swapchain_len];
        self.crosshair_descriptor_sets = self.core.device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(self.descriptor_pool)
                .set_layouts(&layouts)
                .build(),
        )?;
        for (i, &ds) in self.crosshair_descriptor_sets.iter().enumerate() {
            // binding 0: uniform buffer
            let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.text_uniform_buffers[i].buffer())
                .offset(0)
                .range(std::mem::size_of::<UiUniforms>() as vk::DeviceSize)
                .build();
            let buffer_info = [descriptor_buffer_info];
            let buffer_write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(ds)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_info)
                .build();

            // binding 1: sampler
            let sampler_descriptor_image_info = vk::DescriptorImageInfo::builder()
                .image_view(self.textures["crosshair"].view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .sampler(self.graphics_texture_sampler)
                .build();
            let image_info = [sampler_descriptor_image_info];
            let sampler_write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(ds)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&image_info)
                .build();

            self.core.device.update_descriptor_sets(
                &[buffer_write_descriptor_set, sampler_write_descriptor_set],
                &[],
            );
        }

        let layouts = vec![self.selection_descriptor_set_layout.layout(); self.swapchain_len];
        let descriptor_set_alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&layouts)
            .build();

        self.selection_descriptor_sets = self
            .core
            .device
            .allocate_descriptor_sets(&descriptor_set_alloc_info)?;
        for &ds in &self.selection_descriptor_sets {
            // binding 0: sampler
            let sampler_descriptor_image_info = vk::DescriptorImageInfo::builder()
                .image_view(self.textures["selection"].view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .sampler(self.graphics_texture_sampler)
                .build();
            let image_info = [sampler_descriptor_image_info];
            let sampler_write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(ds)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&image_info)
                .build();

            self.core
                .device
                .update_descriptor_sets(&[sampler_write_descriptor_set], &[]);
        }

        Ok(())
    }

    unsafe fn create_color_resources(&mut self) -> VulkanResult<()> {
        let color_format = self.surface_format.format;
        let (color_image, color_image_memory) = self.create_image(
            self.swapchain_extent.width,
            self.swapchain_extent.height,
            1,
            self.msaa_samples,
            color_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        self.color_image = color_image;
        self.color_image_memory = color_image_memory;
        self.color_image_view = self.create_image_view(
            self.color_image,
            color_format,
            1,
            vk::ImageAspectFlags::COLOR,
        )?;

        let command_buffer = OneTimeCommandBuffer::new(
            &self.core.device,
            self.core.graphics_queue,
            self.graphics_command_pool,
        )?;

        self.transition_image_layout(
            &command_buffer,
            self.color_image,
            color_format,
            1,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        )?;

        command_buffer.submit(&[])?;
        Ok(())
    }

    unsafe fn create_depth_resources(&mut self) -> VulkanResult<()> {
        let depth_format = self
            .depth_format()
            .ok_or_else(|| VulkanError::Str("could not find depth format".to_owned()))?;
        let (depth_image, depth_image_memory) = self.create_image(
            self.swapchain_extent.width,
            self.swapchain_extent.height,
            1,
            self.msaa_samples,
            depth_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        self.depth_image_view =
            self.create_image_view(depth_image, depth_format, 1, vk::ImageAspectFlags::DEPTH)?;
        self.depth_image = depth_image;
        self.depth_image_memory = depth_image_memory;
        let command_buffer = OneTimeCommandBuffer::new(
            &self.core.device,
            self.core.graphics_queue,
            self.graphics_command_pool,
        )?;
        self.transition_image_layout(
            &command_buffer,
            self.depth_image,
            depth_format,
            1,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        )?;
        command_buffer.submit(&[])?;
        Ok(())
    }

    unsafe fn find_supported_format(
        &self,
        candidates: &[vk::Format],
        tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> Option<vk::Format> {
        for &format in candidates {
            let props = self
                .core
                .instance
                .get_physical_device_format_properties(self.core.physical_device, format);
            if (tiling == vk::ImageTiling::LINEAR
                && props.linear_tiling_features.contains(features))
                || (tiling == vk::ImageTiling::OPTIMAL
                    && props.optimal_tiling_features.contains(features))
            {
                return Some(format);
            }
        }
        None
    }

    unsafe fn depth_format(&self) -> Option<vk::Format> {
        self.find_supported_format(
            &[
                vk::Format::D32_SFLOAT,
                vk::Format::D32_SFLOAT_S8_UINT,
                vk::Format::D24_UNORM_S8_UINT,
            ],
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        )
    }

    unsafe fn clean_up_swapchain(&mut self) {
        self.core
            .device
            .destroy_image_view(self.color_image_view, None);
        self.core.device.destroy_image(self.color_image, None);
        self.core.device.free_memory(self.color_image_memory, None);
        self.core
            .device
            .destroy_image_view(self.depth_image_view, None);
        self.core.device.destroy_image(self.depth_image, None);
        self.core.device.free_memory(self.depth_image_memory, None);
        for &swapchain_framebuffer in &self.swapchain_framebuffers {
            self.core
                .device
                .destroy_framebuffer(swapchain_framebuffer, None);
        }

        self.core
            .device
            .destroy_pipeline(self.graphics_pipeline, None);
        self.core
            .device
            .destroy_pipeline_layout(self.graphics_pipeline_layout, None);

        self.core.device.destroy_pipeline(self.text_pipeline, None);
        self.core
            .device
            .destroy_pipeline_layout(self.text_pipeline_layout, None);

        self.core
            .device
            .destroy_pipeline(self.selection_pipeline, None);
        self.core
            .device
            .destroy_pipeline_layout(self.selection_pipeline_layout, None);

        self.core
            .device
            .destroy_pipeline(self.crosshair_pipeline, None);
        self.core
            .device
            .destroy_pipeline_layout(self.crosshair_pipeline_layout, None);

        self.core.device.destroy_render_pass(self.render_pass, None);
        for &image_view in self.swapchain_image_views.iter() {
            self.core.device.destroy_image_view(image_view, None);
        }
        self.core
            .swapchain
            .destroy_swapchain(self.swapchain_handle, None);
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {
            self.core
                .device
                .wait_for_fences(&self.in_flight_fences, true, std::u64::MAX)
                .unwrap();

            self.clean_up_swapchain();

            for texture in self.textures.values_mut() {
                texture.deinit();
            }
            for texture in &mut self.glyph_textures {
                texture.deinit();
            }

            self.core
                .device
                .destroy_sampler(self.graphics_texture_sampler, None);
            self.core
                .device
                .destroy_sampler(self.text_texture_sampler, None);
            self.core
                .device
                .destroy_descriptor_pool(self.descriptor_pool, None);

            self.graphics_descriptor_set_layout.deinit(&self.core);
            self.text_descriptor_set_layout.deinit(&self.core);
            self.crosshair_descriptor_set_layout.deinit(&self.core);
            self.selection_descriptor_set_layout.deinit(&self.core);

            for buf in &mut self.text_staging_vertex_buffers {
                buf.deinit();
            }
            for buf in &mut self.text_vertex_buffers {
                buf.deinit();
            }
            for buf in &mut self.text_uniform_buffers {
                buf.deinit();
            }

            for buf in &mut self.selection_staging_vertex_buffers {
                buf.deinit();
            }
            for buf in &mut self.selection_vertex_buffers {
                buf.deinit();
            }

            for buf in &mut self.graphics_staging_vertex_buffers {
                buf.deinit();
            }
            for buf in &mut self.graphics_vertex_buffers {
                buf.deinit();
            }

            self.crosshair_staging_vertex_buffer.deinit();
            self.crosshair_vertex_buffer.deinit();
            for i in 0..self.swapchain_len {
                self.core
                    .device
                    .destroy_semaphore(self.transfer_ownership_semaphores[i], None);
                self.core
                    .device
                    .destroy_semaphore(self.push_const_semaphores[i], None);
            }

            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.core
                    .device
                    .destroy_semaphore(self.render_finished_semaphores[i], None);
                self.core
                    .device
                    .destroy_semaphore(self.image_available_semaphores[i], None);
                self.core
                    .device
                    .destroy_fence(self.in_flight_fences[i], None);
            }

            self.core
                .device
                .destroy_command_pool(self.graphics_command_pool, None);
            self.core
                .device
                .destroy_command_pool(self.transfer_command_pool, None);
        }
    }
}

impl Renderer for VulkanApp {
    fn draw_frame(
        &mut self,
        game_state: &GameState,
        RenderData {
            vertices,
            fps,
            selection_vertices,
        }: &RenderData,
        resized: bool,
    ) -> RendererResult<()> {
        unsafe {
            self.core.device.wait_for_fences(
                &[self.in_flight_fences[self.current_frame]],
                true,
                std::u64::MAX,
            )?;

            self.view_mat = game_state.camera.to_matrix();
            self.uniform_push_constants = UniformPushConstants {
                proj_view: self.proj_mat * self.view_mat,
            };

            if resized {
                self.recreate_swapchain()?;
                println!("recreated swapchain");
                return Ok(());
            }

            match self.core.swapchain.acquire_next_image(
                self.swapchain_handle,
                std::u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null(),
            ) {
                Ok((image_index, _)) => {
                    let image_index = image_index as usize;
                    self.update_text_uniform_buffer(image_index)?;

                    let push_const_cmd_buf = self.new_push_const_cmd_buf(image_index)?;
                    let cmd_bufs = [push_const_cmd_buf];
                    let signal_semaphores = [self.push_const_semaphores[image_index]];
                    self.core.device.queue_submit(
                        self.core.graphics_queue,
                        &[vk::SubmitInfo::builder()
                            .command_buffers(&cmd_bufs)
                            .signal_semaphores(&signal_semaphores)
                            .build()],
                        vk::Fence::null(),
                    )?;

                    // graphics
                    self.graphics_staging_vertex_buffers[image_index].copy_data(vertices)?;

                    // selection
                    if let Some(vertices) = selection_vertices {
                        self.selection_staging_vertex_buffers[image_index].copy_data(vertices)?;
                    }

                    // text
                    self.draw_text(
                        image_index,
                        &format!("{:.0}", fps),
                        Point2f::new(self.screen_width as f32 - 50.0, 10.0),
                        0.5,
                        Color::new(1.0, 0.0, 0.0),
                    )?;

                    let transfer_cmd_buf =
                        self.new_transfer_cmd_buf(image_index, selection_vertices.is_some())?;

                    self.core.device.queue_submit(
                        self.core.transfer_queue,
                        &[vk::SubmitInfo::builder()
                            .command_buffers(&[transfer_cmd_buf])
                            .signal_semaphores(&[self.transfer_ownership_semaphores[image_index]])
                            .build()],
                        vk::Fence::null(),
                    )?;

                    let command_buffers = [self.take_ownership_cmd_buffers[image_index]];
                    let wait_semaphores = [self.transfer_ownership_semaphores[image_index]];
                    let wait_dst_stage_mask = [vk::PipelineStageFlags::VERTEX_INPUT];
                    self.core.device.queue_submit(
                        self.core.graphics_queue,
                        &[vk::SubmitInfo::builder()
                            .command_buffers(&command_buffers)
                            .wait_semaphores(&wait_semaphores)
                            .wait_dst_stage_mask(&wait_dst_stage_mask)
                            .build()],
                        vk::Fence::null(),
                    )?;

                    let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];

                    // Start render pass
                    let wait_semaphores = [
                        self.image_available_semaphores[self.current_frame],
                        self.push_const_semaphores[image_index],
                    ];
                    let wait_dst_stage_mask = [
                        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        vk::PipelineStageFlags::VERTEX_SHADER,
                    ];

                    let text_draw_cmd_buf = self.new_text_draw_cmd_buf(image_index)?;
                    let graphics_draw_cmd_buf = self.new_graphics_draw_cmd_buf(image_index)?;
                    let crosshair_draw_cmd_buf = self.crosshair_draw_cmd_bufs[image_index];
                    let draw_cmd_bufs = if selection_vertices.is_some() {
                        vec![
                            graphics_draw_cmd_buf,
                            text_draw_cmd_buf,
                            crosshair_draw_cmd_buf,
                            self.new_selection_draw_cmd_buf(image_index)?,
                        ]
                    } else {
                        vec![
                            graphics_draw_cmd_buf,
                            text_draw_cmd_buf,
                            crosshair_draw_cmd_buf,
                        ]
                    };

                    let command_buffers =
                        [self.new_render_pass_command_buffer(image_index, &draw_cmd_bufs)?];
                    let submit_info = vk::SubmitInfo::builder()
                        .wait_semaphores(&wait_semaphores)
                        .wait_dst_stage_mask(&wait_dst_stage_mask)
                        .command_buffers(&command_buffers)
                        .signal_semaphores(&signal_semaphores)
                        .build();

                    self.core
                        .device
                        .reset_fences(&[self.in_flight_fences[self.current_frame]])?;
                    self.core.device.queue_submit(
                        self.core.graphics_queue,
                        &[submit_info],
                        self.in_flight_fences[self.current_frame],
                    )?;

                    let swapchains = [self.swapchain_handle];
                    let image_indices = [image_index as u32];
                    let present_info = vk::PresentInfoKHR::builder()
                        .wait_semaphores(&signal_semaphores)
                        .swapchains(&swapchains)
                        .image_indices(&image_indices)
                        .build();

                    match self
                        .core
                        .swapchain
                        .queue_present(self.core.graphics_queue, &present_info)
                    {
                        Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                            self.recreate_swapchain()?
                        }
                        Ok(false) => (),
                        Err(e) => return Err(e.into()),
                    }
                    self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
                    Ok(())
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.recreate_swapchain()?;
                    Ok(())
                }
                Err(e) => Err(e.into()),
            }
        }
    }

    fn events_loop(&mut self) -> &mut EventsLoop {
        &mut self.core.events_loop
    }

    fn window(&self) -> &Window {
        &self.core.window
    }
}
