use ash;
use ash::{
    extensions::{DebugUtils, Surface, Swapchain, Win32Surface},
    prelude::VkResult,
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0, InstanceV1_1},
    vk, vk_make_version, Device, Entry, Instance,
};
use byteorder::ByteOrder;
use byteorder::LittleEndian;
use crate::na::Vector2;
use crate::types::*;
use crate::utils::clamp;
use failure_derive::Fail;
use std::{
    ffi::{CStr, CString},
    fs::File,
    io::prelude::*,
    os::raw::*,
    path::Path,
    ptr,
};
#[cfg(target_os = "windows")]
use winapi;
use winit;
use winit::{dpi::LogicalSize, DeviceEvent, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

type Vector2f = Vector2<f32>;

const MAX_FRAMES_IN_FLIGHT: usize = 2;

fn from_vk_result<T>(result: vk::Result, t: T) -> VkResult<T> {
    match result {
        vk::Result::SUCCESS => Ok(t),
        e => Err(e),
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
) -> VkResult<()> {
    let s = CString::new("vkCreateDebugUtilsMessengerEXT").unwrap();
    if let Some(func) = entry.get_instance_proc_addr(instance, s.as_ptr()) {
        let func: vk::PFN_vkCreateDebugUtilsMessengerEXT = std::mem::transmute(func);
        from_vk_result(func(instance, create_info, alloc_callbacks, callback), ())
    } else {
        Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT)
    }
}

unsafe fn destroy_debug_utils_messenger_ext(
    entry: &Entry,
    instance: vk::Instance,
    callback: vk::DebugUtilsMessengerEXT,
    alloc_callbacks: *const vk::AllocationCallbacks,
) {
    let s = CString::new("vkDestroyDebugUtilsMessengerEXT").unwrap();
    if let Some(func) = entry.get_instance_proc_addr(instance, s.as_ptr()) {
        let func: vk::PFN_vkDestroyDebugUtilsMessengerEXT = std::mem::transmute(func);
        func(instance, callback, alloc_callbacks);
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

#[derive(Clone, Copy)]
struct Vertex {
    pub pos: Vector2f,
    pub color: Vector3f,
}

impl Vertex {
    fn convert_vec_f32(vertices: &[Vertex]) -> Vec<f32> {
        let mut v = vec![];
        for vtx in vertices {
            v.extend(vtx.pos.as_slice());
            v.extend(vtx.color.as_slice());
        }
        v
    }

    fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(0)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(std::mem::size_of::<Vector2f>() as u32)
                .build(),
        ]
    }
}

#[derive(Fail, Debug)]
pub enum VulkanError {
    #[fail(display = "{}", _0)]
    NulError(#[cause] std::ffi::NulError),
    #[fail(display = "{}", _0)]
    Str(String),
    #[fail(display = "{}", _0)]
    Io(#[cause] std::io::Error),
    #[fail(display = "{}", _0)]
    ImageError(#[cause] image::ImageError),
    #[fail(display = "{}", _0)]
    SystemTimeError(#[cause] std::time::SystemTimeError),
    #[fail(display = "{}", _0)]
    VkError(#[cause] vk::Result),
}

impl From<vk::Result> for VulkanError {
    fn from(err: vk::Result) -> VulkanError {
        VulkanError::VkError(err)
    }
}

pub struct VulkanBase {
    entry: Entry,
    instance: Instance,
    device: Device,
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

    graphics_pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    graphics_pipeline: vk::Pipeline,

    graphics_command_pool: vk::CommandPool,
    graphics_command_buffers: Vec<vk::CommandBuffer>,
    transfer_command_pool: vk::CommandPool,

    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,

    current_frame: usize,

    vertices: Vec<Vertex>,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    indices: Vec<u16>,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,

    // debug
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl VulkanBase {
    pub fn new(screen_width: u32, screen_height: u32) -> Result<VulkanBase, VulkanError> {
        let events_loop = winit::EventsLoop::new();
        let window = winit::WindowBuilder::new()
            .with_title("Minecrust")
            .with_resizable(false)
            .with_dimensions(LogicalSize::new(
                f64::from(screen_width),
                f64::from(screen_height),
            ))
            .build(&events_loop)
            .unwrap();
        let entry = Entry::new().unwrap();
        let app_name = CString::new("VulkanTriangle").unwrap();
        let raw_name = app_name.as_ptr();
        let layer_names = [CString::new("VK_LAYER_LUNARG_standard_validation").unwrap()];
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
                .shader_clip_distance(true)
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
            let device = instance
                .create_device(physical_device, &device_ci, None)
                .unwrap();
            let swapchain = Swapchain::new(&instance, &device);
            let graphics_queue = device.get_device_queue(graphics_queue_family_index, 0);
            let transfer_queue = device.get_device_queue(transfer_queue_family_index, 0);

            let mut base = VulkanBase {
                current_frame: 0,
                events_loop,
                entry,
                instance,
                device,
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
                graphics_pipeline_layout: Default::default(),
                graphics_pipeline: Default::default(),
                graphics_command_pool: Default::default(),
                graphics_command_buffers: Default::default(),
                transfer_command_pool: Default::default(),
                image_available_semaphores: Default::default(),
                render_finished_semaphores: Default::default(),
                in_flight_fences: Default::default(),
                vertices: Default::default(),
                vertex_buffer: Default::default(),
                vertex_buffer_memory: Default::default(),
                indices: Default::default(),
                index_buffer: Default::default(),
                index_buffer_memory: Default::default(),
            };

            base.create_swapchain(screen_width, screen_height)?;
            base.create_render_pass()?;
            base.create_graphics_pipeline()?;

            base.create_framebuffers()?;
            base.create_command_pools()?;

            base.vertices = vec![
                Vertex {
                    pos: Vector2f::new(-0.5, -0.5),
                    color: Vector3f::new(1.0, 0.0, 0.0),
                },
                Vertex {
                    pos: Vector2f::new(0.5, -0.5),
                    color: Vector3f::new(0.0, 1.0, 0.0),
                },
                Vertex {
                    pos: Vector2f::new(0.5, 0.5),
                    color: Vector3f::new(0.0, 0.0, 1.0),
                },
                Vertex {
                    pos: Vector2f::new(-0.5, 0.5),
                    color: Vector3f::new(1.0, 1.0, 1.0),
                },
            ];
            base.indices = vec![0, 1, 2, 2, 3, 0];

            base.create_vertex_buffer()?;
            base.create_index_buffer()?;
            base.create_command_buffers()?;
            base.create_sync_objects()?;
            Ok(base)
        }
    }

    unsafe fn create_swapchain(&mut self, screen_width: u32, screen_height: u32) -> VkResult<()> {
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

        let surface_capabilities = self
            .surface
            .get_physical_device_surface_capabilities_khr(self.physical_device, self.surface_handle)
            .unwrap();
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
        self.swapchain_image_views = self
            .swapchain_images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo {
                    s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                    p_next: ptr::null(),
                    flags: Default::default(),
                    view_type: vk::ImageViewType::TYPE_2D,
                    format: self.surface_format.format,
                    components: vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    },
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    image,
                };
                self.device.create_image_view(&create_view_info, None)
            })
            .collect::<Result<Vec<vk::ImageView>, vk::Result>>()?;
        Ok(())
    }

    unsafe fn create_render_pass(&mut self) -> VkResult<()> {
        let color_attachment_description = vk::AttachmentDescription::builder()
            .format(self.surface_format.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build();
        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();
        let subpass_description = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&[color_attachment_ref])
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
            .attachments(&[color_attachment_description])
            .subpasses(&[subpass_description])
            .dependencies(&[subpass_dependency])
            .build();
        self.render_pass = self.device.create_render_pass(&render_pass_ci, None)?;
        Ok(())
    }

    unsafe fn create_graphics_pipeline(&mut self) -> VkResult<()> {
        let vert_module = self.create_shader_module_from_file("src/shaders/triangle-vert.spv")?;
        let frag_module = self.create_shader_module_from_file("src/shaders/triangle-frag.spv")?;
        let shader_stage_name = CString::new("main").unwrap();
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

        let binding_description = Vertex::binding_description();
        let attribute_descriptions = Vertex::attribute_descriptions();

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
            .max_depth(0.0)
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
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .build();
        let multisample_state_ci = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
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
        let graphics_pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&[])
            .push_constant_ranges(&[])
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
                .attachments(&[swapchain_image_view])
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
            .build();
        self.graphics_command_pool = self
            .device
            .create_command_pool(&graphics_command_pool_ci, None)?;
        let transfer_command_pool_ci = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(self.transfer_queue_family_index)
            .build();
        self.transfer_command_pool = self
            .device
            .create_command_pool(&transfer_command_pool_ci, None)?;
        Ok(())
    }

    unsafe fn create_command_buffers(&mut self) -> VkResult<()> {
        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.graphics_command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(self.swapchain_framebuffers.len() as u32)
            .build();
        self.graphics_command_buffers = self
            .device
            .allocate_command_buffers(&command_buffer_alloc_info)?;
        for (index, &command_buffer) in self.graphics_command_buffers.iter().enumerate() {
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE)
                .build();
            self.device
                .begin_command_buffer(command_buffer, &begin_info)?;
            let swapchain_framebuffer = self.swapchain_framebuffers[index];
            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.render_pass)
                .framebuffer(swapchain_framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain_extent,
                })
                .clear_values(&[vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 0.0],
                    },
                }])
                .build();
            self.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );

            self.device
                .cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer], &[0]);
            self.device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer,
                0,
                vk::IndexType::UINT16,
            );

            self.device
                .cmd_draw_indexed(command_buffer, self.indices.len() as u32, 1, 0, 0, 0);
            self.device.cmd_end_render_pass(command_buffer);
            self.device.end_command_buffer(command_buffer)?;
        }
        Ok(())
    }

    unsafe fn recreate_swapchain(&mut self) -> VkResult<()> {
        self.device.device_wait_idle()?;
        self.clean_up_swapchain();
        let LogicalSize { width, height } = self.window.get_inner_size().unwrap();
        self.create_swapchain(width as u32, height as u32)?;

        self.create_render_pass()?;
        self.create_graphics_pipeline()?;
        self.create_framebuffers()?;
        self.create_command_buffers()?;

        Ok(())
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

    unsafe fn create_vertex_buffer(&mut self) -> VkResult<()> {
        let buffer_size: vk::DeviceSize =
            (self.vertices.len() * std::mem::size_of::<Vertex>()) as vk::DeviceSize;

        let (staging_buffer, staging_buffer_memory) = self.create_buffer(
            buffer_size as vk::DeviceSize,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = self.device.map_memory(
            staging_buffer_memory,
            0,
            buffer_size as vk::DeviceSize,
            vk::MemoryMapFlags::empty(),
        )? as *mut f32;
        let vertices_f32 = Vertex::convert_vec_f32(&self.vertices);
        std::ptr::copy_nonoverlapping(vertices_f32.as_ptr(), data, vertices_f32.len());
        self.device.unmap_memory(staging_buffer_memory);

        let (vertex_buffer, vertex_buffer_memory) = self.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        self.copy_buffer(staging_buffer, vertex_buffer, buffer_size)?;
        self.device.destroy_buffer(staging_buffer, None);
        self.device.free_memory(staging_buffer_memory, None);

        self.vertex_buffer = vertex_buffer;
        self.vertex_buffer_memory = vertex_buffer_memory;

        Ok(())
    }

    unsafe fn create_index_buffer(&mut self) -> VkResult<()> {
        let buffer_size = (std::mem::size_of::<u16>() * self.indices.len()) as vk::DeviceSize;
        let (staging_buffer, staging_buffer_memory) = self.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = self.device.map_memory(
            staging_buffer_memory,
            0,
            buffer_size as vk::DeviceSize,
            vk::MemoryMapFlags::empty(),
        )? as *mut u16;
        std::ptr::copy_nonoverlapping(self.indices.as_ptr(), data, self.indices.len());
        self.device.unmap_memory(staging_buffer_memory);

        let (index_buffer, index_buffer_memory) = self.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        self.copy_buffer(staging_buffer, index_buffer, buffer_size)?;
        self.device.destroy_buffer(staging_buffer, None);
        self.device.free_memory(staging_buffer_memory, None);

        self.index_buffer = index_buffer;
        self.index_buffer_memory = index_buffer_memory;

        Ok(())
    }

    unsafe fn create_sync_objects(&mut self) -> VkResult<()> {
        let semaphore_ci = vk::SemaphoreCreateInfo::default();
        let fence_ci = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED)
            .build();
        self.image_available_semaphores = vec![];
        self.render_finished_semaphores = vec![];
        self.in_flight_fences = vec![];
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

    unsafe fn copy_buffer(
        &self,
        src: vk::Buffer,
        dst: vk::Buffer,
        size: vk::DeviceSize,
    ) -> VkResult<()> {
        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.transfer_command_pool)
            .command_buffer_count(1)
            .build();
        let command_buffer = self
            .device
            .allocate_command_buffers(&command_buffer_alloc_info)?[0];

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .build();
        self.device
            .begin_command_buffer(command_buffer, &command_buffer_begin_info)?;
        let copy_region = vk::BufferCopy::builder().size(size).build();
        self.device
            .cmd_copy_buffer(command_buffer, src, dst, &[copy_region]);
        self.device.end_command_buffer(command_buffer)?;

        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&[command_buffer])
            .build();
        let fence_ci = vk::FenceCreateInfo::default();
        let fence = self.device.create_fence(&fence_ci, None)?;
        self.device
            .queue_submit(self.transfer_queue, &[submit_info], fence)?;
        self.device.wait_for_fences(&[fence], true, std::u64::MAX)?;
        self.device.destroy_fence(fence, None);

        self.device
            .free_command_buffers(self.transfer_command_pool, &[command_buffer]);

        Ok(())
    }

    unsafe fn create_shader_module_from_file<P: AsRef<Path>>(
        &self,
        file: P,
    ) -> VkResult<vk::ShaderModule> {
        let mut shader_spv_file = File::open(file).unwrap();
        let mut buf = vec![];
        shader_spv_file.read_to_end(&mut buf).unwrap();
        self.create_shader_module(&buf)
    }

    unsafe fn create_shader_module(&self, code: &[u8]) -> VkResult<vk::ShaderModule> {
        let mut code_u32: Vec<u32> = vec![0; code.len() / 4];
        LittleEndian::read_u32_into(code, &mut code_u32);
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(&code_u32)
            .build();
        self.device.create_shader_module(&create_info, None)
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

    unsafe fn draw_frame(&mut self, resized: bool) -> VkResult<()> {
        self.device.wait_for_fences(
            &[self.in_flight_fences[self.current_frame]],
            true,
            std::u64::MAX,
        )?;

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
                let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];
                let submit_info = vk::SubmitInfo::builder()
                    .wait_semaphores(&[self.image_available_semaphores[self.current_frame]])
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                    .command_buffers(&[self.graphics_command_buffers[image_index as usize]])
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
                    .image_indices(&[image_index])
                    .build();

                match self
                    .swapchain
                    .queue_present_khr(self.graphics_queue, &present_info)
                {
                    Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                        self.recreate_swapchain()?
                    }
                    Ok(false) => (),
                    Err(e) => return Err(e),
                }
                self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
                Ok(())
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self.recreate_swapchain(),
            Err(e) => Err(e),
        }
    }

    pub fn start(&mut self) {
        let mut running = true;
        let mut just_started = true;
        while running {
            let mut resized = false;
            self.events_loop.poll_events(|event| match event {
                Event::DeviceEvent { event, .. } => match event {
                    DeviceEvent::Key(KeyboardInput {
                        virtual_keycode: Some(keycode),
                        ..
                    }) => {
                        if keycode == VirtualKeyCode::Escape {
                            running = false;
                        }
                    }
                    _ => (),
                },
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => running = false,
                    WindowEvent::Resized(LogicalSize { width, height }) => {
                        // FIXME: handle minimization?
                        // Right now we don't allow the window to be resized
                        if just_started {
                            // When the window is first created, a resized event is sent
                            just_started = false;
                        } else {
                            println!("resized to ({}, {})", width, height);
                            resized = true;
                        }
                    }
                    _ => (),
                },
                _ => (),
            });
            unsafe {
                let _ = self.draw_frame(resized);
            }
        }
    }

    unsafe fn clean_up_swapchain(&mut self) {
        for &swapchain_framebuffer in &self.swapchain_framebuffers {
            self.device.destroy_framebuffer(swapchain_framebuffer, None);
        }
        self.device
            .free_command_buffers(self.graphics_command_pool, &self.graphics_command_buffers);
        self.device.destroy_pipeline(self.graphics_pipeline, None);
        self.device
            .destroy_pipeline_layout(self.graphics_pipeline_layout, None);
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

            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);
            self.device.destroy_buffer(self.index_buffer, None);
            self.device.free_memory(self.index_buffer_memory, None);

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
            );
            self.instance.destroy_instance(None);
        }
    }
}
