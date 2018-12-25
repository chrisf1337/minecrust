pub mod error;
mod one_time_command_buffer;
mod texture;
mod vertex_buffer;

use crate::{
    freetype,
    game::GameState,
    na::geometry::Perspective3,
    renderer::{RenderData, Renderer, RendererResult},
    types::*,
    utils::{clamp, NSEC_PER_SEC},
    vulkan::{
        error::{VulkanError, VulkanResult},
        one_time_command_buffer::OneTimeCommandBuffer,
        texture::Texture,
        vertex_buffer::VertexBuffer,
    },
};
use ash;
use ash::{
    extensions::{DebugUtils, Surface, Swapchain, Win32Surface},
    prelude::VkResult,
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0, InstanceV1_1},
    vk, vk_make_version, Device, Entry, Instance,
};
use byteorder::ByteOrder;
use byteorder::LittleEndian;
use image;
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    fs::File,
    io::prelude::*,
    os::raw::*,
    path::Path,
    ptr,
    rc::Rc,
    time::SystemTime,
};
#[cfg(target_os = "windows")]
use winapi;
use winit;
use winit::{dpi::LogicalSize, EventsLoop, Window};

const MAX_FRAMES_IN_FLIGHT: usize = 5;
const VERTEX_BUFFER_CAPCITY: vk::DeviceSize = 1 << 20;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct UniformBufferObject {
    pub model: Transform3f,
    pub view: Matrix4f,
    pub proj: Matrix4f,
}

impl UniformBufferObject {
    fn to_vec(&self) -> Vec<f32> {
        let mut v = vec![];
        v.extend(self.model.to_homogeneous().as_slice());
        v.extend(self.view.as_slice());
        v.extend(self.proj.as_slice());
        v
    }
}

impl Default for UniformBufferObject {
    fn default() -> Self {
        UniformBufferObject {
            model: Transform3f::identity(),
            view: Matrix4f::identity(),
            proj: Matrix4f::identity(),
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

fn from_vk_result<T>(result: vk::Result, t: T) -> VulkanResult<T> {
    match result {
        vk::Result::SUCCESS => Ok(t),
        e => Err(e.into()),
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    _message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> u32 {
    println!("{:?}", CStr::from_ptr((*p_callback_data).p_message));
    vk::FALSE
}

unsafe fn create_debug_utils_messenger_ext(
    entry: &Entry,
    instance: vk::Instance,
    create_info: &vk::DebugUtilsMessengerCreateInfoEXT,
    alloc_callbacks: *const vk::AllocationCallbacks,
    callback: *mut vk::DebugUtilsMessengerEXT,
) -> VulkanResult<()> {
    let s = CString::new("vkCreateDebugUtilsMessengerEXT")?;
    if let Some(func) = entry.get_instance_proc_addr(instance, s.as_ptr()) {
        let func: vk::PFN_vkCreateDebugUtilsMessengerEXT = std::mem::transmute(func);
        from_vk_result(func(instance, create_info, alloc_callbacks, callback), ())
    } else {
        Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT.into())
    }
}

unsafe fn destroy_debug_utils_messenger_ext(
    entry: &Entry,
    instance: vk::Instance,
    callback: vk::DebugUtilsMessengerEXT,
    alloc_callbacks: *const vk::AllocationCallbacks,
) -> VulkanResult<()> {
    let s = CString::new("vkDestroyDebugUtilsMessengerEXT")?;
    if let Some(func) = entry.get_instance_proc_addr(instance, s.as_ptr()) {
        let func: vk::PFN_vkDestroyDebugUtilsMessengerEXT = std::mem::transmute(func);
        func(instance, callback, alloc_callbacks);
        Ok(())
    } else {
        Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT.into())
    }
}

/// Returns (physical_device, graphics queue, transfer queue)
unsafe fn find_queue_families(
    instance: &Instance,
    surface: &Surface,
    surface_handle: vk::SurfaceKHR,
) -> VkResult<(vk::PhysicalDevice, Option<u32>, Option<u32>)> {
    let mut graphics_queue_family_index = None;
    let mut transfer_queue_family_index = None;
    for pd in instance.enumerate_physical_devices()? {
        for (index, &queue_family_properties) in instance
            .get_physical_device_queue_family_properties(pd)
            .iter()
            .enumerate()
        {
            if graphics_queue_family_index.is_none()
                && queue_family_properties
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
                && surface.get_physical_device_surface_support_khr(pd, index as u32, surface_handle)
            {
                graphics_queue_family_index = Some(index as u32);
            } else if transfer_queue_family_index.is_none()
                && queue_family_properties
                    .queue_flags
                    .contains(vk::QueueFlags::TRANSFER)
                && !queue_family_properties
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
            {
                transfer_queue_family_index = Some(index as u32);
            }
            if graphics_queue_family_index.is_some() && transfer_queue_family_index.is_some() {
                return Ok((pd, graphics_queue_family_index, transfer_queue_family_index));
            }
        }
    }
    Ok((vk::PhysicalDevice::null(), None, None))
}

unsafe fn has_stencil_component(format: vk::Format) -> bool {
    format == vk::Format::D32_SFLOAT_S8_UINT || format == vk::Format::D24_UNORM_S8_UINT
}

#[cfg(target_os = "windows")]
unsafe fn create_surface<E: EntryV1_0, I: InstanceV1_1>(
    entry: &E,
    instance: &I,
    window: &winit::Window,
) -> Result<vk::SurfaceKHR, vk::Result> {
    use winapi::shared::windef::HWND;
    use winapi::um::libloaderapi::GetModuleHandleW;
    use winit::os::windows::WindowExt;

    let hwnd = window.get_hwnd() as HWND;
    let hinstance = GetModuleHandleW(ptr::null()) as *const c_void;
    let win32_create_info = vk::Win32SurfaceCreateInfoKHR {
        s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
        p_next: ptr::null(),
        flags: Default::default(),
        hinstance,
        hwnd: hwnd as *const c_void,
    };
    let win32_surface = Win32Surface::new(entry, instance);
    win32_surface.create_win32_surface_khr(&win32_create_info, None)
}

fn pad_font_bytes(bytes: Vec<u8>) -> Vec<u8> {
    let mut v = vec![];
    for b in bytes {
        v.push(b);
        v.push(b);
        v.push(b);
        v.push(255);
    }
    v
}

pub struct VulkanBase {
    entry: Entry,
    instance: Instance,
    device: Rc<Device>,
    surface: Surface,
    window: winit::Window,
    events_loop: winit::EventsLoop,

    physical_device: vk::PhysicalDevice,
    graphics_queue: vk::Queue,
    graphics_queue_family_index: u32,
    transfer_queue: vk::Queue,
    transfer_queue_family_index: u32,

    surface_handle: vk::SurfaceKHR,
    surface_format: vk::SurfaceFormatKHR,

    swapchain: Swapchain,
    swapchain_extent: vk::Extent2D,
    swapchain_handle: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    swapchain_framebuffers: Vec<vk::Framebuffer>,
    swapchain_len: usize,

    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    render_pass: vk::RenderPass,

    glyph_metrics: HashMap<char, freetype::GlyphMetrics>,

    graphics_command_pool: vk::CommandPool,
    graphics_command_buffers: Vec<vk::CommandBuffer>,
    transfer_command_pool: vk::CommandPool,
    transfer_command_buffers: Vec<vk::CommandBuffer>,

    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,

    current_frame: usize,

    graphics_pipeline: vk::Pipeline,
    graphics_pipeline_layout: vk::PipelineLayout,

    graphics_staging_vertex_buffers: Vec<VertexBuffer<Vertex3f>>,
    graphics_vertex_buffers: Vec<VertexBuffer<TextVertex>>,
    graphics_draw_cmd_bufs: Vec<Option<vk::CommandBuffer>>,

    text_pipeline: vk::Pipeline,
    text_pipeline_layout: vk::PipelineLayout,

    text_staging_vertex_buffers: Vec<VertexBuffer<TextVertex>>,
    text_vertex_buffers: Vec<VertexBuffer<TextVertex>>,
    text_draw_cmd_bufs: Vec<Option<vk::CommandBuffer>>,

    transfer_cmd_bufs: Vec<Option<vk::CommandBuffer>>,
    take_ownership_cmd_buffers: Vec<vk::CommandBuffer>,
    transfer_ownership_semaphores: Vec<vk::Semaphore>,

    uniform_buffers: Vec<vk::Buffer>,
    uniform_buffers_memory: Vec<vk::DeviceMemory>,

    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,

    textures: HashMap<String, Texture>,
    texture_sampler: vk::Sampler,

    depth_image: vk::Image,
    depth_image_memory: vk::DeviceMemory,
    depth_image_view: vk::ImageView,

    msaa_samples: vk::SampleCountFlags,

    color_image: vk::Image,
    color_image_memory: vk::DeviceMemory,
    color_image_view: vk::ImageView,

    start_time: SystemTime,

    view_mat: Matrix4f,
    proj_mat: Matrix4f,
    uniform_push_constants: UniformPushConstants,

    semaphore: vk::Semaphore,

    // debug
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl VulkanBase {
    pub fn new(screen_width: u32, screen_height: u32) -> VulkanResult<VulkanBase> {
        let events_loop = winit::EventsLoop::new();
        let window = winit::WindowBuilder::new()
            .with_title("Minecrust")
            .with_resizable(true)
            .with_dimensions(LogicalSize::new(
                f64::from(screen_width),
                f64::from(screen_height),
            ))
            .build(&events_loop)
            .unwrap();
        let entry = Entry::new().unwrap();
        let app_name = CString::new("VulkanTriangle")?;
        let raw_name = app_name.as_ptr();
        let layer_names = [CString::new("VK_LAYER_LUNARG_standard_validation")?];
        let layer_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();
        let extension_names_raw = vec![
            Surface::name().as_ptr(),
            Win32Surface::name().as_ptr(),
            DebugUtils::name().as_ptr(),
        ];
        let app_info = vk::ApplicationInfo {
            p_application_name: raw_name,
            s_type: vk::StructureType::APPLICATION_INFO,
            p_next: ptr::null(),
            application_version: 0,
            p_engine_name: raw_name,
            engine_version: 0,
            api_version: vk_make_version!(1, 1, 0),
        };
        let instance_ci = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_next: ptr::null(),
            flags: Default::default(),
            p_application_info: &app_info,
            pp_enabled_layer_names: layer_names_raw.as_ptr(),
            enabled_layer_count: layer_names_raw.len() as u32,
            pp_enabled_extension_names: extension_names_raw.as_ptr(),
            enabled_extension_count: extension_names_raw.len() as u32,
        };
        unsafe {
            let instance = entry
                .create_instance(&instance_ci, None)
                .expect("Instance creation error");
            let debug_messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT {
                s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
                p_next: ptr::null(),
                flags: Default::default(),
                message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                message_type: vk::DebugUtilsMessageTypeFlagsEXT::all(),
                pfn_user_callback: Some(vulkan_debug_callback),
                p_user_data: ptr::null_mut(),
            };
            let mut debug_messenger: vk::DebugUtilsMessengerEXT = Default::default();
            create_debug_utils_messenger_ext(
                &entry,
                instance.handle(),
                &debug_messenger_create_info,
                ptr::null(),
                &mut debug_messenger,
            )?;

            let surface_handle = create_surface(&entry, &instance, &window)?;
            let surface = Surface::new(&entry, &instance);
            let (physical_device, graphics_queue_family_index, transfer_queue_family_index) = {
                if let (
                    physical_device,
                    Some(graphics_queue_family_index),
                    Some(transfer_queue_family_index),
                ) = find_queue_families(&instance, &surface, surface_handle)?
                {
                    (
                        physical_device,
                        graphics_queue_family_index,
                        transfer_queue_family_index,
                    )
                } else {
                    return Err(VulkanError::Str(
                        "cannot find suitable queue family".to_string(),
                    ));
                }
            };
            let device_extension_names_raw = [Swapchain::name().as_ptr()];
            let features = vk::PhysicalDeviceFeatures::builder()
                .sampler_anisotropy(true)
                .shader_clip_distance(true)
                .sample_rate_shading(true)
                .build();
            let graphics_queue_ci = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(graphics_queue_family_index)
                .queue_priorities(&[1.0])
                .build();
            let transfer_queue_ci = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(transfer_queue_family_index)
                .queue_priorities(&[1.0])
                .build();
            let device_ci = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&[graphics_queue_ci, transfer_queue_ci])
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features)
                .build();
            let device = instance.create_device(physical_device, &device_ci, None)?;
            let swapchain = Swapchain::new(&instance, &device);
            let graphics_queue = device.get_device_queue(graphics_queue_family_index, 0);
            let transfer_queue = device.get_device_queue(transfer_queue_family_index, 0);

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

            let semaphore_ci = vk::SemaphoreCreateInfo::default();
            let semaphore = device.create_semaphore(&semaphore_ci, None)?;

            let mut base = VulkanBase {
                current_frame: 0,
                events_loop,
                entry,
                instance,
                device: Rc::new(device),
                physical_device,
                graphics_queue,
                graphics_queue_family_index,
                transfer_queue,
                transfer_queue_family_index,
                window,
                swapchain,
                surface,
                surface_handle,
                debug_messenger,

                surface_format: Default::default(),
                render_pass: Default::default(),
                swapchain_extent: Default::default(),
                swapchain_handle: Default::default(),
                swapchain_images: Default::default(),
                swapchain_image_views: Default::default(),
                swapchain_framebuffers: Default::default(),
                swapchain_len: 0,

                descriptor_set_layouts: Default::default(),

                graphics_pipeline_layout: Default::default(),
                graphics_pipeline: Default::default(),
                text_pipeline_layout: Default::default(),
                text_pipeline: Default::default(),

                glyph_metrics: Default::default(),

                graphics_command_pool: Default::default(),
                graphics_command_buffers: Default::default(),
                transfer_command_pool: Default::default(),
                transfer_command_buffers: Default::default(),

                image_available_semaphores: Default::default(),
                render_finished_semaphores: Default::default(),
                in_flight_fences: Default::default(),

                graphics_staging_vertex_buffers: Default::default(),
                graphics_vertex_buffers: Default::default(),
                graphics_draw_cmd_bufs: Default::default(),

                text_staging_vertex_buffers: Default::default(),
                text_vertex_buffers: Default::default(),
                text_draw_cmd_bufs: Default::default(),

                transfer_cmd_bufs: Default::default(),
                take_ownership_cmd_buffers: Default::default(),
                transfer_ownership_semaphores: Default::default(),

                uniform_buffers: Default::default(),
                uniform_buffers_memory: Default::default(),

                descriptor_pool: Default::default(),
                descriptor_sets: Default::default(),

                textures: HashMap::default(),
                texture_sampler: Default::default(),

                depth_image: Default::default(),
                depth_image_memory: Default::default(),
                depth_image_view: Default::default(),

                color_image: Default::default(),
                color_image_memory: Default::default(),
                color_image_view: Default::default(),

                msaa_samples: Default::default(),

                start_time: SystemTime::now(),

                semaphore,

                view_mat,
                proj_mat,
                uniform_push_constants: UniformPushConstants {
                    proj_view: proj_mat * view_mat,
                },
            };

            base.msaa_samples = base.get_max_usable_sample_count();
            base.texture_sampler = base.create_texture_sampler()?;

            base.create_swapchain(screen_width, screen_height)?;

            base.graphics_command_buffers = vec![Default::default(); base.swapchain_len];
            base.transfer_command_buffers = vec![Default::default(); base.swapchain_len];

            base.create_render_pass()?;
            base.create_descriptor_set_layout()?;
            base.create_graphics_pipeline()?;
            base.create_text_pipeline()?;

            base.create_command_pools()?;

            base.create_color_resources()?;
            base.create_depth_resources()?;
            base.create_framebuffers()?;

            // fonts
            let ft_lib = freetype::Library::new()?;
            let mut face = ft_lib.new_face("assets/fonts/SourceCodePro-Regular.ttf", 0)?;
            face.set_pixel_sizes(0, 48)?;

            for c in 0..=255u8 {
                let c = c as char;
                face.load_char(c, freetype::LoadFlags::Render)?;
                let glyph = face.glyph.as_ref().unwrap();
                let bitmap = &glyph.bitmap;
                assert_eq!(glyph.metrics.width as u32, bitmap.width);
                let (buffer, dimensions) = if !bitmap.buffer.is_empty() {
                    (bitmap.buffer.clone(), (bitmap.width, bitmap.rows))
                } else {
                    (vec![0], (1, 1))
                };
                let texture =
                    base.create_texture_image_from_bytes(&pad_font_bytes(buffer), dimensions, 1)?;
                base.textures.insert(c.to_string(), texture);
            }

            let texture = base.create_texture_image("assets/texture.jpg")?;
            let cobblestone_texture =
                base.create_texture_image("assets/cobblestone-border-arrow.png")?;
            base.textures.insert("texture".to_string(), texture);
            base.textures
                .insert("cobblestone".to_string(), cobblestone_texture);

            base.create_uniform_buffers()?;
            base.create_descriptor_pool()?;
            base.create_descriptor_sets()?;
            base.create_sync_objects()?;

            base.create_text_buffers()?;

            Ok(base)
        }
    }

    unsafe fn get_max_usable_sample_count(&self) -> vk::SampleCountFlags {
        let physical_device_properties = self
            .instance
            .get_physical_device_properties(self.physical_device);
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
        let surface_formats = self
            .surface
            .get_physical_device_surface_formats_khr(self.physical_device, self.surface_handle)?;
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

        let surface_capabilities = self.surface.get_physical_device_surface_capabilities_khr(
            self.physical_device,
            self.surface_handle,
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
        let present_modes = self.surface.get_physical_device_surface_present_modes_khr(
            self.physical_device,
            self.surface_handle,
        )?;
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        let swapchain_ci = vk::SwapchainCreateInfoKHR::builder()
            .surface(self.surface_handle)
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
        self.swapchain_handle = self.swapchain.create_swapchain_khr(&swapchain_ci, None)?;
        self.swapchain_images = self
            .swapchain
            .get_swapchain_images_khr(self.swapchain_handle)?;
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

    unsafe fn create_text_pipeline(&mut self) -> VulkanResult<()> {
        let vert_module = self.create_shader_module_from_file("src/shaders/text-vert.spv")?;
        let frag_module = self.create_shader_module_from_file("src/shaders/text-frag.spv")?;
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

        let vertex_input_state_ci = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&[TextVertex::binding_description()])
            .vertex_attribute_descriptions(&TextVertex::attribute_descriptions())
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
        let viewport_state_ci = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&[viewport])
            .scissors(&[scissor])
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
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
            .build();
        let color_blend_state_ci = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&[color_blend_attachment])
            .build();
        let depth_stencil_state_ci = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);
        let push_constant_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .size(std::mem::size_of::<UniformPushConstants>() as u32)
            .offset(0)
            .build();
        let graphics_pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&self.descriptor_set_layouts)
            .push_constant_ranges(&[push_constant_range])
            .build();
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&graphics_pipeline_layout_ci, None)?;

        let pipeline_ci = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_state_ci)
            .input_assembly_state(&input_assembly_state_ci)
            .viewport_state(&viewport_state_ci)
            .rasterization_state(&rasterizer_state_ci)
            .multisample_state(&multisample_state_ci)
            .color_blend_state(&color_blend_state_ci)
            .depth_stencil_state(&depth_stencil_state_ci)
            .layout(self.graphics_pipeline_layout)
            .render_pass(self.render_pass)
            .subpass(0)
            .build();
        let pipeline = self
            .device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_ci], None)
            .map_err(|x| x.1)?[0];

        // clean up shader modules
        self.device.destroy_shader_module(vert_module, None);
        self.device.destroy_shader_module(frag_module, None);

        self.text_pipeline_layout = pipeline_layout;
        self.text_pipeline = pipeline;

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
            .ok_or_else(|| VulkanError::Str("couldn't find depth format".to_string()))?;
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

        let subpass_description = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&[color_attachment_ref])
            .depth_stencil_attachment(&depth_attachment_ref)
            .resolve_attachments(&[color_attachment_resolve_ref])
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
        let render_pass_ci = vk::RenderPassCreateInfo::builder()
            .attachments(&[
                color_attachment_description,
                depth_attachment_description,
                color_attachment_resolve,
            ])
            .subpasses(&[subpass_description])
            .dependencies(&[subpass_dependency])
            .build();
        self.render_pass = self.device.create_render_pass(&render_pass_ci, None)?;
        Ok(())
    }

    unsafe fn create_descriptor_set_layout(&mut self) -> VkResult<()> {
        let ubo_descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build();
        let sampler_layout_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .immutable_samplers(&[self.texture_sampler])
            .build();
        let texture_layout_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(2)
            .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();
        let descriptor_set_layout_ci = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&[
                ubo_descriptor_set_layout_binding,
                sampler_layout_binding,
                texture_layout_binding,
            ])
            .build();

        for _ in &self.swapchain_images {
            self.descriptor_set_layouts.push(
                self.device
                    .create_descriptor_set_layout(&descriptor_set_layout_ci, None)?,
            );
        }

        Ok(())
    }

    unsafe fn create_graphics_pipeline(&mut self) -> VulkanResult<()> {
        let vert_module = self.create_shader_module_from_file("src/shaders/triangle-vert.spv")?;
        let frag_module = self.create_shader_module_from_file("src/shaders/triangle-frag.spv")?;
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

        let binding_description = Vertex3f::binding_description();
        let attribute_descriptions = Vertex3f::attribute_descriptions();

        let vertex_input_state_ci = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&[binding_description])
            .vertex_attribute_descriptions(&attribute_descriptions)
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
        let viewport_state_ci = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&[viewport])
            .scissors(&[scissor])
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
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(false)
            .build();
        let color_blend_state_ci = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&[color_blend_attachment])
            .build();
        // let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::LINE_WIDTH];
        // let dynamic_state_ci = vk::PipelineDynamicStateCreateInfo::builder()
        //     .dynamic_states(&dynamic_states)
        //     .build();
        let depth_stencil_state_ci = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);
        let push_constant_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .size(std::mem::size_of::<UniformPushConstants>() as u32)
            .offset(0)
            .build();
        let graphics_pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&self.descriptor_set_layouts)
            .push_constant_ranges(&[push_constant_range])
            .build();
        self.graphics_pipeline_layout = self
            .device
            .create_pipeline_layout(&graphics_pipeline_layout_ci, None)?;

        let pipeline_ci = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_state_ci)
            .input_assembly_state(&input_assembly_state_ci)
            .viewport_state(&viewport_state_ci)
            .rasterization_state(&rasterizer_state_ci)
            .multisample_state(&multisample_state_ci)
            .color_blend_state(&color_blend_state_ci)
            .depth_stencil_state(&depth_stencil_state_ci)
            .layout(self.graphics_pipeline_layout)
            .render_pass(self.render_pass)
            .subpass(0)
            .build();
        self.graphics_pipeline = self
            .device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_ci], None)
            .map_err(|x| x.1)?[0];

        // clean up shader modules
        self.device.destroy_shader_module(vert_module, None);
        self.device.destroy_shader_module(frag_module, None);

        Ok(())
    }

    unsafe fn create_framebuffers(&mut self) -> VkResult<()> {
        self.swapchain_framebuffers = vec![];
        for &swapchain_image_view in &self.swapchain_image_views {
            let framebuffer_ci = vk::FramebufferCreateInfo::builder()
                .render_pass(self.render_pass)
                .attachments(&[
                    self.color_image_view,
                    self.depth_image_view,
                    swapchain_image_view,
                ])
                .width(self.swapchain_extent.width)
                .height(self.swapchain_extent.height)
                .layers(1)
                .build();
            let framebuffer = self.device.create_framebuffer(&framebuffer_ci, None)?;
            self.swapchain_framebuffers.push(framebuffer);
        }
        Ok(())
    }

    unsafe fn create_command_pools(&mut self) -> VkResult<()> {
        let graphics_command_pool_ci = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(self.graphics_queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .build();
        self.graphics_command_pool = self
            .device
            .create_command_pool(&graphics_command_pool_ci, None)?;
        let transfer_command_pool_ci = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(self.transfer_queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .build();
        self.transfer_command_pool = self
            .device
            .create_command_pool(&transfer_command_pool_ci, None)?;
        Ok(())
    }

    unsafe fn new_render_pass_command_buffer(
        &mut self,
        index: usize,
        secondary_command_buffers: &[vk::CommandBuffer],
    ) -> VkResult<vk::CommandBuffer> {
        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.graphics_command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1)
            .build();
        let cmd_buf = self
            .device
            .allocate_command_buffers(&command_buffer_alloc_info)?[0];
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE)
            .build();
        self.device.begin_command_buffer(cmd_buf, &begin_info)?;

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(self.swapchain_framebuffers[index])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain_extent,
            })
            .clear_values(&[
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
            ])
            .build();

        self.device.cmd_begin_render_pass(
            cmd_buf,
            &render_pass_begin_info,
            vk::SubpassContents::SECONDARY_COMMAND_BUFFERS,
        );

        self.device
            .cmd_execute_commands(cmd_buf, secondary_command_buffers);

        self.device.cmd_end_render_pass(cmd_buf);
        self.device.end_command_buffer(cmd_buf)?;

        Ok(cmd_buf)
    }

    unsafe fn create_text_take_ownership_cmd_bufs(&mut self) -> VkResult<()> {
        for index in 0..self.swapchain_len {
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(self.graphics_command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1)
                .build();
            let command_buffer = self
                .device
                .allocate_command_buffers(&command_buffer_alloc_info)?[0];

            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE)
                .build();
            self.device
                .begin_command_buffer(command_buffer, &begin_info)?;

            self.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::VERTEX_INPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[vk::BufferMemoryBarrier::builder()
                    .src_access_mask(vk::AccessFlags::empty())
                    .dst_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ)
                    .src_queue_family_index(self.transfer_queue_family_index)
                    .dst_queue_family_index(self.graphics_queue_family_index)
                    .buffer(self.graphics_vertex_buffers[index].buffer)
                    .build()],
                &[],
            );
            self.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::VERTEX_INPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[vk::BufferMemoryBarrier::builder()
                    .src_access_mask(vk::AccessFlags::empty())
                    .dst_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ)
                    .src_queue_family_index(self.transfer_queue_family_index)
                    .dst_queue_family_index(self.graphics_queue_family_index)
                    .buffer(self.text_vertex_buffers[index].buffer)
                    .build()],
                &[],
            );

            self.device.end_command_buffer(command_buffer)?;

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
            self.device
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
                vk::CommandBufferUsageFlags::SIMULTANEOUS_USE
                    | vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE,
            )
            .build();
        self.device.begin_command_buffer(cmd_buf, &begin_info)?;

        self.device
            .cmd_bind_pipeline(cmd_buf, vk::PipelineBindPoint::GRAPHICS, self.text_pipeline);
        self.device.cmd_bind_vertex_buffers(
            cmd_buf,
            0,
            &[self.text_vertex_buffers[index].buffer],
            &[0],
        );
        self.device.cmd_draw(
            cmd_buf,
            self.text_staging_vertex_buffers[index].len as u32,
            1,
            0,
            0,
        );

        self.device.end_command_buffer(cmd_buf)?;

        self.text_draw_cmd_bufs[index] = Some(cmd_buf);

        Ok(cmd_buf)
    }

    unsafe fn update_text_vertex_buffer(&mut self, index: usize, vertices: &[TextVertex]) {
        self.text_staging_vertex_buffers[index].copy_data(vertices);
    }

    unsafe fn new_text_transfer_cmd_buf(&mut self, index: usize) -> VkResult<vk::CommandBuffer> {
        let cmd_buf = if let Some(cmd_buf) = self.transfer_cmd_bufs[index] {
            cmd_buf
        } else {
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(self.transfer_command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1)
                .build();
            self.device
                .allocate_command_buffers(&command_buffer_alloc_info)?[0]
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE)
            .build();
        self.device.begin_command_buffer(cmd_buf, &begin_info)?;

        self.device.cmd_copy_buffer(
            cmd_buf,
            self.graphics_staging_vertex_buffers[index].buffer,
            self.graphics_vertex_buffers[index].buffer,
            // FIXME: need to change this when buf_len increases
            &[vk::BufferCopy::builder()
                .size(self.graphics_staging_vertex_buffers[index].buf_len)
                .build()],
        );
        self.device.cmd_copy_buffer(
            cmd_buf,
            self.text_staging_vertex_buffers[index].buffer,
            self.text_vertex_buffers[index].buffer,
            // FIXME: need to change this when buf_len increases
            &[vk::BufferCopy::builder()
                .size(self.text_staging_vertex_buffers[index].buf_len)
                .build()],
        );

        self.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[vk::BufferMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::empty())
                .src_queue_family_index(self.transfer_queue_family_index)
                .dst_queue_family_index(self.graphics_queue_family_index)
                .buffer(self.graphics_vertex_buffers[index].buffer)
                .build()],
            &[],
        );
        self.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[vk::BufferMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::empty())
                .src_queue_family_index(self.transfer_queue_family_index)
                .dst_queue_family_index(self.graphics_queue_family_index)
                .buffer(self.text_vertex_buffers[index].buffer)
                .build()],
            &[],
        );

        self.device.end_command_buffer(cmd_buf)?;

        self.transfer_cmd_bufs.push(Some(cmd_buf));

        Ok(cmd_buf)
    }

    unsafe fn create_text_buffers(&mut self) -> VkResult<()> {
        for _ in 0..self.swapchain_len {
            let mut staging_buf = VertexBuffer::new(
                self,
                VERTEX_BUFFER_CAPCITY,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            staging_buf.map()?;
            self.graphics_staging_vertex_buffers.push(staging_buf);

            self.graphics_vertex_buffers.push(VertexBuffer::new(
                self,
                VERTEX_BUFFER_CAPCITY,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?);

            let mut staging_buf = VertexBuffer::new(
                self,
                VERTEX_BUFFER_CAPCITY,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            staging_buf.map()?;
            self.text_staging_vertex_buffers.push(staging_buf);

            self.text_vertex_buffers.push(VertexBuffer::new(
                self,
                VERTEX_BUFFER_CAPCITY,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?);
        }

        self.graphics_draw_cmd_bufs = vec![Default::default(); self.swapchain_len];
        self.text_draw_cmd_bufs = vec![Default::default(); self.swapchain_len];

        self.transfer_cmd_bufs = vec![Default::default(); self.swapchain_len];
        self.create_text_take_ownership_cmd_bufs()?;

        Ok(())
    }

    unsafe fn draw_text(
        &mut self,
        index: usize,
        string: &str,
        pos: Point2f,
        scale: f32,
        color: Color,
    ) {
        let vertex_buffer = &mut self.text_staging_vertex_buffers[index];
        let mut vertices: Vec<TextVertex> = vec![];
    }

    pub fn recreate_swapchain(&mut self) -> VulkanResult<()> {
        unsafe {
            self.device.device_wait_idle()?;
            self.clean_up_swapchain();
            let LogicalSize { width, height } = self.window.get_inner_size().unwrap();
            self.create_swapchain(width as u32, height as u32)?;

            self.create_render_pass()?;
            self.create_graphics_pipeline()?;
            self.create_text_pipeline()?;
            self.create_color_resources()?;
            self.create_depth_resources()?;
            self.create_framebuffers()?;

            let mut flip_mat = Matrix4f::from_diagonal(&Vector4f::new(1.0, -1.0, 0.5, 1.0));
            flip_mat[(2, 3)] = 0.5;
            self.proj_mat = flip_mat
                * Perspective3::new(
                    width as f32 / height as f32,
                    f32::to_radians(45.0),
                    0.1,
                    100.0,
                )
                .to_homogeneous();
            self.uniform_push_constants = UniformPushConstants {
                proj_view: self.proj_mat * self.view_mat,
            };

            Ok(())
        }
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
            self.device
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
                vk::CommandBufferUsageFlags::SIMULTANEOUS_USE
                    | vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE,
            )
            .build();
        self.device.begin_command_buffer(cmd_buf, &begin_info)?;

        self.device.cmd_bind_pipeline(
            cmd_buf,
            vk::PipelineBindPoint::GRAPHICS,
            self.graphics_pipeline,
        );
        self.device.cmd_bind_vertex_buffers(
            cmd_buf,
            0,
            &[self.graphics_vertex_buffers[index].buffer],
            &[0],
        );
        self.device.cmd_bind_descriptor_sets(
            cmd_buf,
            vk::PipelineBindPoint::GRAPHICS,
            self.graphics_pipeline_layout,
            0,
            &self.descriptor_sets,
            &[],
        );

        self.device.cmd_draw(
            cmd_buf,
            self.graphics_staging_vertex_buffers[index].len as u32,
            1,
            0,
            0,
        );

        self.device.end_command_buffer(cmd_buf)?;

        self.graphics_draw_cmd_bufs[index] = Some(cmd_buf);

        Ok(cmd_buf)
    }

    unsafe fn update_graphics_vertex_buffer(&mut self, index: usize, vertices: &[Vertex3f]) {
        self.graphics_staging_vertex_buffers[index].copy_data(vertices);
    }

    unsafe fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> VkResult<(vk::Buffer, vk::DeviceMemory)> {
        let buffer_ci = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();
        let buffer = self.device.create_buffer(&buffer_ci, None)?;

        let memory_requirements = self.device.get_buffer_memory_requirements(buffer);
        let memory_alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(
                self.find_memory_type(memory_requirements.memory_type_bits, memory_property_flags)
                    .unwrap(),
            )
            .build();
        let buffer_memory = self.device.allocate_memory(&memory_alloc_info, None)?;
        self.device.bind_buffer_memory(buffer, buffer_memory, 0)?;
        Ok((buffer, buffer_memory))
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
                width: width,
                height: height,
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
        let image = self.device.create_image(&image_ci, None)?;
        let memory_requirements = self.device.get_image_memory_requirements(image);
        let memory_alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(
                self.find_memory_type(memory_requirements.memory_type_bits, memory_properties)
                    .unwrap(),
            )
            .build();
        let image_memory = self.device.allocate_memory(&memory_alloc_info, None)?;
        self.device.bind_image_memory(image, image_memory, 0)?;
        Ok((image, image_memory))
    }

    unsafe fn create_texture_image_from_bytes(
        &self,
        bytes: &[u8],
        (img_width, img_height): (u32, u32),
        mip_levels: u32,
    ) -> VulkanResult<Texture> {
        let img_size = vk::DeviceSize::from(img_width * img_height * 4);
        let (staging_buffer, staging_buffer_memory) = self.create_buffer(
            img_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        let data = self.device.map_memory(
            staging_buffer_memory,
            0,
            img_size,
            vk::MemoryMapFlags::empty(),
        )? as *mut u8;

        assert_eq!(bytes.len(), img_size as usize);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
        self.device.unmap_memory(staging_buffer_memory);

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
            &self.device,
            self.graphics_queue,
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

        self.device.destroy_buffer(staging_buffer, None);
        self.device.free_memory(staging_buffer_memory, None);

        let texture_image_view = self.create_texture_image_view(texture_image, mip_levels)?;

        Ok(Texture {
            image: texture_image,
            view: texture_image_view,
            memory: texture_image_memory,
        })
    }

    unsafe fn create_texture_image<P: AsRef<Path>>(&mut self, path: P) -> VulkanResult<Texture> {
        let img = image::open(path)?.to_rgba();
        let (img_width, img_height) = img.dimensions();
        let mip_levels = f32::floor(f32::log2(u32::max(img_width, img_height) as f32)) as u32 + 1;

        self.create_texture_image_from_bytes(&img.to_vec(), (img_width, img_height), mip_levels)
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
        Ok(self.device.create_image_view(&image_view_ci, None)?)
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
            .instance
            .get_physical_device_format_properties(self.physical_device, image_format);
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
            &self.device,
            self.graphics_queue,
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
            self.device.cmd_pipeline_barrier(
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
            self.device.cmd_blit_image(
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

            self.device.cmd_pipeline_barrier(
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

        self.device.cmd_pipeline_barrier(
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

    unsafe fn create_texture_sampler(&self) -> VulkanResult<vk::Sampler> {
        let sampler_ci = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::NEAREST)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(false)
            .max_anisotropy(16.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(100.0)
            .build();
        Ok(self.device.create_sampler(&sampler_ci, None)?)
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
            return Err(VulkanError::Str(
                "unsupported layout transition".to_string(),
            ));
        }

        let barrier = barrier_builder.build();
        self.device.cmd_pipeline_barrier(
            command_buffer.command_buffer,
            source_stage,
            destination_stage,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
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
        let region = vk::BufferImageCopy::builder()
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
            .build();
        self.device.cmd_copy_buffer_to_image(
            command_buffer.command_buffer,
            buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[region],
        );

        Ok(())
    }

    unsafe fn create_uniform_buffers(&mut self) -> VkResult<()> {
        let buffer_size = std::mem::size_of::<UniformBufferObject>() as vk::DeviceSize;
        for _ in 0..self.swapchain_len {
            let (uniform_buffer, uniform_buffer_memory) = self.create_buffer(
                buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            self.uniform_buffers.push(uniform_buffer);
            self.uniform_buffers_memory.push(uniform_buffer_memory);
        }
        Ok(())
    }

    unsafe fn create_sync_objects(&mut self) -> VkResult<()> {
        let semaphore_ci = vk::SemaphoreCreateInfo::builder().build();
        let fence_ci = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED)
            .build();
        self.image_available_semaphores = vec![];
        self.render_finished_semaphores = vec![];
        self.in_flight_fences = vec![];

        for _ in 0..self.swapchain_len {
            self.transfer_ownership_semaphores
                .push(self.device.create_semaphore(&semaphore_ci, None)?);
        }

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            self.image_available_semaphores
                .push(self.device.create_semaphore(&semaphore_ci, None)?);
            self.render_finished_semaphores
                .push(self.device.create_semaphore(&semaphore_ci, None)?);
            self.in_flight_fences
                .push(self.device.create_fence(&fence_ci, None)?);
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
        Ok(self.device.create_shader_module(&create_info, None)?)
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
            .instance
            .get_physical_device_memory_properties(self.physical_device);
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

    unsafe fn update_uniform_buffer(&self, current_image: usize) -> VkResult<()> {
        let time = SystemTime::now().duration_since(self.start_time).unwrap();
        let time_f = time.as_nanos() as f32 / NSEC_PER_SEC as f32;
        let mut ubo = UniformBufferObject::default();
        ubo.model = Transform3f::identity();
        // ubo.model =
        //     Rotation3f::from_axis_angle(&Vector3f::y_axis(), time_f * std::f32::consts::FRAC_PI_2)
        //         .to_superset();
        ubo.view = Matrix4f::look_at_rh(
            &Point3f::new(0.0, 1.0, 1.0),
            &Point3f::origin(),
            &Vector3f::y_axis(),
        );
        let LogicalSize { width, height } = self.window.get_inner_size().unwrap();
        // See https://matthewwellings.com/blog/the-new-vulkan-coordinate-system/
        let mut flip_mat = Matrix4f::from_diagonal(&Vector4f::new(1.0, -1.0, 0.5, 1.0));
        flip_mat[(2, 3)] = 0.5;
        ubo.proj = flip_mat
            * Perspective3::new(
                width as f32 / height as f32,
                f32::to_radians(45.0),
                0.1,
                100.0,
            )
            .to_homogeneous();

        let data = self.device.map_memory(
            self.uniform_buffers_memory[current_image],
            0,
            std::mem::size_of::<UniformBufferObject>() as vk::DeviceSize,
            vk::MemoryMapFlags::empty(),
        )? as *mut f32;
        let v = ubo.to_vec();
        std::ptr::copy_nonoverlapping(v.as_ptr(), data, v.len());
        self.device
            .unmap_memory(self.uniform_buffers_memory[current_image]);

        Ok(())
    }

    unsafe fn create_descriptor_pool(&mut self) -> VkResult<()> {
        let uniforms_pool_size = vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count((self.swapchain_len * 2) as u32)
            .build();
        let sampler_pool_size = vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::SAMPLER)
            .descriptor_count((self.swapchain_len * 2) as u32)
            .build();
        let texture_pool_size = vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::SAMPLED_IMAGE)
            .descriptor_count(self.swapchain_len as u32)
            .build();
        let descriptor_pool_ci = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&[uniforms_pool_size, sampler_pool_size, texture_pool_size])
            .max_sets((self.swapchain_len * 2) as u32)
            .build();
        self.descriptor_pool = self
            .device
            .create_descriptor_pool(&descriptor_pool_ci, None)?;
        Ok(())
    }

    unsafe fn create_descriptor_sets(&mut self) -> VkResult<()> {
        let descriptor_set_alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&self.descriptor_set_layouts)
            .build();
        self.descriptor_sets = self
            .device
            .allocate_descriptor_sets(&descriptor_set_alloc_info)?;

        for (index, &uniform_buffer) in self.uniform_buffers.iter().enumerate() {
            let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(uniform_buffer)
                .offset(0)
                .range(std::mem::size_of::<UniformBufferObject>() as vk::DeviceSize)
                .build();
            let buffer_write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(self.descriptor_sets[index])
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&[descriptor_buffer_info])
                .build();

            // binding 1: sampler
            let sampler_descriptor_image_info = vk::DescriptorImageInfo::builder()
                .sampler(self.texture_sampler)
                .build();
            let sampler_write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(self.descriptor_sets[index])
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(&[sampler_descriptor_image_info])
                .build();

            // binding 2: image
            let cobblestone_texture = self.textures["cobblestone"];
            let texture_descriptor_image_info = vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(cobblestone_texture.view)
                .build();
            let texture_write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(self.descriptor_sets[index])
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(&[texture_descriptor_image_info])
                .build();

            self.device.update_descriptor_sets(
                &[
                    buffer_write_descriptor_set,
                    sampler_write_descriptor_set,
                    texture_write_descriptor_set,
                ],
                &[],
            );
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
            &self.device,
            self.graphics_queue,
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
            .ok_or_else(|| VulkanError::Str("could not find depth format".to_string()))?;
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
            &self.device,
            self.graphics_queue,
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
                .instance
                .get_physical_device_format_properties(self.physical_device, format);
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
        self.device.destroy_image_view(self.color_image_view, None);
        self.device.destroy_image(self.color_image, None);
        self.device.free_memory(self.color_image_memory, None);
        self.device.destroy_image_view(self.depth_image_view, None);
        self.device.destroy_image(self.depth_image, None);
        self.device.free_memory(self.depth_image_memory, None);
        for &swapchain_framebuffer in &self.swapchain_framebuffers {
            self.device.destroy_framebuffer(swapchain_framebuffer, None);
        }
        self.device
            .free_command_buffers(self.graphics_command_pool, &self.graphics_command_buffers);

        self.device.destroy_pipeline(self.graphics_pipeline, None);
        self.device
            .destroy_pipeline_layout(self.graphics_pipeline_layout, None);

        self.device.destroy_pipeline(self.text_pipeline, None);
        self.device
            .destroy_pipeline_layout(self.text_pipeline_layout, None);

        self.device.destroy_render_pass(self.render_pass, None);
        for &image_view in self.swapchain_image_views.iter() {
            self.device.destroy_image_view(image_view, None);
        }
        self.swapchain
            .destroy_swapchain_khr(self.swapchain_handle, None);
    }
}

impl Drop for VulkanBase {
    fn drop(&mut self) {
        unsafe {
            self.device
                .wait_for_fences(&self.in_flight_fences, true, std::u64::MAX)
                .unwrap();

            self.clean_up_swapchain();

            for texture in self.textures.values() {
                self.device.destroy_image_view(texture.view, None);
                self.device.destroy_image(texture.image, None);
                self.device.free_memory(texture.memory, None);
            }

            self.device.destroy_sampler(self.texture_sampler, None);
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            for &descriptor_set_layout in &self.descriptor_set_layouts {
                self.device
                    .destroy_descriptor_set_layout(descriptor_set_layout, None);
            }
            for &uniform_buffer in &self.uniform_buffers {
                self.device.destroy_buffer(uniform_buffer, None);
            }
            for &uniform_buffer_memory in &self.uniform_buffers_memory {
                self.device.free_memory(uniform_buffer_memory, None);
            }

            for buf in &mut self.text_staging_vertex_buffers {
                buf.deinit();
            }
            for buf in &mut self.text_vertex_buffers {
                buf.deinit();
            }
            for buf in &mut self.graphics_staging_vertex_buffers {
                buf.deinit();
            }
            for buf in &mut self.graphics_vertex_buffers {
                buf.deinit();
            }

            self.device.destroy_semaphore(self.semaphore, None);

            for i in 0..self.swapchain_len {
                self.device
                    .destroy_semaphore(self.transfer_ownership_semaphores[i], None);
            }

            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device
                    .destroy_semaphore(self.render_finished_semaphores[i], None);
                self.device
                    .destroy_semaphore(self.image_available_semaphores[i], None);
                self.device.destroy_fence(self.in_flight_fences[i], None);
            }

            self.device
                .destroy_command_pool(self.graphics_command_pool, None);
            self.device
                .destroy_command_pool(self.transfer_command_pool, None);
            self.device.destroy_device(None);
            self.surface.destroy_surface_khr(self.surface_handle, None);
            destroy_debug_utils_messenger_ext(
                &self.entry,
                self.instance.handle(),
                self.debug_messenger,
                ptr::null(),
            )
            .unwrap();
            self.instance.destroy_instance(None);
        }
    }
}

impl Renderer for VulkanBase {
    fn draw_frame(
        &mut self,
        game_state: &GameState,
        render_data: &RenderData,
        resized: bool,
    ) -> RendererResult<()> {
        unsafe {
            self.device.wait_for_fences(
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

            match self.swapchain.acquire_next_image_khr(
                self.swapchain_handle,
                std::u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null(),
            ) {
                Ok((image_index, _)) => {
                    let image_index = image_index as usize;
                    self.update_uniform_buffer(image_index)?;

                    let mut data = [0u8; std::mem::size_of::<UniformPushConstants>()];
                    LittleEndian::write_f32_into(&self.uniform_push_constants.to_vec(), &mut data);
                    let command_buffer = OneTimeCommandBuffer::new(
                        &self.device,
                        self.graphics_queue,
                        self.graphics_command_pool,
                    )?;
                    self.device.cmd_push_constants(
                        command_buffer.command_buffer,
                        self.graphics_pipeline_layout,
                        vk::ShaderStageFlags::VERTEX,
                        0,
                        &data,
                    );
                    command_buffer.submit(&[self.semaphore])?;

                    // graphics
                    self.update_graphics_vertex_buffer(image_index, &render_data.vertices);

                    // text
                    self.update_text_vertex_buffer(
                        image_index,
                        &[
                            TextVertex::new(
                                0,
                                Point2f::new(-0.5, -0.5),
                                Point2f::new(0.0, 0.0),
                                Color::new(1.0, 0.0, 0.0),
                            ),
                            TextVertex::new(
                                0,
                                Point2f::new(-0.5, 0.5),
                                Point2f::new(0.0, 0.0),
                                Color::new(1.0, 0.0, 0.0),
                            ),
                            TextVertex::new(
                                0,
                                Point2f::new(0.5, -0.5),
                                Point2f::new(0.0, 0.0),
                                Color::new(1.0, 0.0, 0.0),
                            ),
                        ],
                    );

                    let text_transfer_cmd_buf = self.new_text_transfer_cmd_buf(image_index)?;
                    let text_draw_cmd_buf = self.new_text_draw_cmd_buf(image_index)?;
                    let graphics_draw_cmd_buf = self.new_graphics_draw_cmd_buf(image_index)?;

                    self.device.queue_submit(
                        self.transfer_queue,
                        &[vk::SubmitInfo::builder()
                            .command_buffers(&[text_transfer_cmd_buf])
                            .signal_semaphores(&[self.transfer_ownership_semaphores[image_index]])
                            .build()],
                        vk::Fence::null(),
                    )?;
                    self.device.queue_submit(
                        self.graphics_queue,
                        &[vk::SubmitInfo::builder()
                            .command_buffers(&[self.take_ownership_cmd_buffers[image_index]])
                            .wait_semaphores(&[self.transfer_ownership_semaphores[image_index]])
                            .wait_dst_stage_mask(&[vk::PipelineStageFlags::VERTEX_INPUT])
                            .build()],
                        vk::Fence::null(),
                    )?;

                    let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];

                    // Start render pass
                    let submit_info = vk::SubmitInfo::builder()
                        .wait_semaphores(&[
                            self.image_available_semaphores[self.current_frame],
                            self.semaphore,
                        ])
                        .wait_dst_stage_mask(&[
                            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                            vk::PipelineStageFlags::VERTEX_SHADER,
                        ])
                        .command_buffers(&[self.new_render_pass_command_buffer(
                            image_index,
                            &[graphics_draw_cmd_buf, text_draw_cmd_buf],
                        )?])
                        .signal_semaphores(&signal_semaphores)
                        .build();

                    self.device
                        .reset_fences(&[self.in_flight_fences[self.current_frame]])?;
                    self.device.queue_submit(
                        self.graphics_queue,
                        &[submit_info],
                        self.in_flight_fences[self.current_frame],
                    )?;

                    let swapchains = [self.swapchain_handle];
                    let present_info = vk::PresentInfoKHR::builder()
                        .wait_semaphores(&signal_semaphores)
                        .swapchains(&swapchains)
                        .image_indices(&[image_index as u32])
                        .build();

                    match self
                        .swapchain
                        .queue_present_khr(self.graphics_queue, &present_info)
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
        &mut self.events_loop
    }

    fn window(&self) -> &Window {
        &self.window
    }
}
