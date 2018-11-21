use ash;
use ash::{
    extensions::{DebugUtils, Surface, Swapchain, Win32Surface},
    prelude::VkResult,
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0, InstanceV1_1},
    vk, vk_make_version, Device, Entry, Instance,
};
use byteorder::ByteOrder;
use byteorder::LittleEndian;
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
    let win32_surface_loader = Win32Surface::new(entry, instance);
    win32_surface_loader.create_win32_surface_khr(&win32_create_info, None)
}

pub struct VulkanBase {
    pub entry: Entry,
    pub instance: Instance,
    pub device: Device,
    pub surface_loader: Surface,
    pub window: winit::Window,
    pub events_loop: winit::EventsLoop,

    pub pdevice: vk::PhysicalDevice,
    pub queue_family_index: u32,
    pub graphics_queue: vk::Queue,

    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,

    pub swapchain: Swapchain,
    pub swapchain_extent: vk::Extent2D,
    pub swapchain_handle: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub swapchain_framebuffers: Vec<vk::Framebuffer>,

    pub graphics_pipeline_layout: vk::PipelineLayout,
    pub render_pass: vk::RenderPass,
    pub graphics_pipeline: vk::Pipeline,

    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,

    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,

    pub current_frame: usize,

    // debug
    pub debug_messenger: vk::DebugUtilsMessengerEXT,
}

unsafe fn create_shader_module_from_file<P: AsRef<Path>>(
    device: &Device,
    file: P,
) -> VkResult<vk::ShaderModule> {
    let mut shader_spv_file = File::open(file).unwrap();
    let mut buf = vec![];
    shader_spv_file.read_to_end(&mut buf).unwrap();
    create_shader_module(device, &buf)
}

unsafe fn create_shader_module(device: &Device, code: &[u8]) -> VkResult<vk::ShaderModule> {
    let mut code_u32: Vec<u32> = vec![0; code.len() / 4];
    LittleEndian::read_u32_into(code, &mut code_u32);
    let create_info = vk::ShaderModuleCreateInfo::builder()
        .code(&code_u32)
        .build();
    device.create_shader_module(&create_info, None)
}

impl VulkanBase {
    pub fn new(screen_width: u32, screen_height: u32) -> VkResult<VulkanBase> {
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
        let create_info = vk::InstanceCreateInfo {
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
                .create_instance(&create_info, None)
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

            let surface = create_surface(&entry, &instance, &window)?;
            let pdevices = instance.enumerate_physical_devices()?;
            let surface_loader = Surface::new(&entry, &instance);
            let (pdevice, queue_family_index) = pdevices
                .iter()
                .map(|pdevice| {
                    instance
                        .get_physical_device_queue_family_properties(*pdevice)
                        .iter()
                        .enumerate()
                        .filter_map(|(index, info)| {
                            println!("{:?}", info);
                            if info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                && surface_loader.get_physical_device_surface_support_khr(
                                    *pdevice,
                                    index as u32,
                                    surface,
                                ) {
                                Some((*pdevice, index as u32))
                            } else {
                                None
                            }
                        })
                        .nth(0)
                })
                .filter_map(|v| v)
                .nth(0)
                .expect("No physical device");
            let device_extension_names_raw = [Swapchain::name().as_ptr()];
            let features = vk::PhysicalDeviceFeatures {
                shader_clip_distance: 1,
                ..Default::default()
            };
            let priorities = [1.0];
            let queue_ci = vk::DeviceQueueCreateInfo {
                s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
                p_next: ptr::null(),
                flags: Default::default(),
                queue_family_index,
                p_queue_priorities: priorities.as_ptr(),
                queue_count: priorities.len() as u32,
            };
            let device_ci = vk::DeviceCreateInfo {
                s_type: vk::StructureType::DEVICE_CREATE_INFO,
                p_next: ptr::null(),
                flags: Default::default(),
                queue_create_info_count: 1,
                p_queue_create_infos: &queue_ci,
                enabled_layer_count: 0,
                pp_enabled_layer_names: ptr::null(),
                enabled_extension_count: device_extension_names_raw.len() as u32,
                pp_enabled_extension_names: device_extension_names_raw.as_ptr(),
                p_enabled_features: &features,
            };
            let device = instance.create_device(pdevice, &device_ci, None).unwrap();
            let graphics_queue = device.get_device_queue(queue_family_index, 0);
            let surface_formats =
                surface_loader.get_physical_device_surface_formats_khr(pdevice, surface)?;
            let surface_format = surface_formats
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
            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities_khr(pdevice, surface)
                .unwrap();
            let mut desired_image_count = surface_capabilities.min_image_count + 1;
            if surface_capabilities.max_image_count > 0
                && desired_image_count > surface_capabilities.max_image_count
            {
                desired_image_count = surface_capabilities.max_image_count;
            }
            let swapchain_extent = if surface_capabilities.current_extent.width == std::u32::MAX {
                vk::Extent2D {
                    width: screen_width,
                    height: screen_height,
                }
            } else {
                surface_capabilities.current_extent
            };
            let pre_transform = if surface_capabilities
                .supported_transforms
                .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
            {
                vk::SurfaceTransformFlagsKHR::IDENTITY
            } else {
                surface_capabilities.current_transform
            };
            let present_modes =
                surface_loader.get_physical_device_surface_present_modes_khr(pdevice, surface)?;
            let present_mode = present_modes
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);
            let swapchain = Swapchain::new(&instance, &device);
            let swapchain_ci = vk::SwapchainCreateInfoKHR {
                s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
                p_next: ptr::null(),
                flags: Default::default(),
                surface,
                min_image_count: desired_image_count,
                image_color_space: surface_format.color_space,
                image_format: surface_format.format,
                image_extent: swapchain_extent,
                image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                image_sharing_mode: vk::SharingMode::EXCLUSIVE,
                pre_transform,
                composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
                present_mode,
                clipped: vk::TRUE,
                old_swapchain: vk::SwapchainKHR::null(),
                image_array_layers: 1,
                p_queue_family_indices: ptr::null(),
                queue_family_index_count: 0,
            };
            let swapchain_handle = swapchain.create_swapchain_khr(&swapchain_ci, None)?;
            let swapchain_images = swapchain.get_swapchain_images_khr(swapchain_handle)?;
            let swapchain_image_views: Vec<vk::ImageView> = swapchain_images
                .iter()
                .map(|&image| {
                    let create_view_info = vk::ImageViewCreateInfo {
                        s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                        p_next: ptr::null(),
                        flags: Default::default(),
                        view_type: vk::ImageViewType::TYPE_2D,
                        format: surface_format.format,
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
                    device.create_image_view(&create_view_info, None)
                })
                .collect::<Result<Vec<vk::ImageView>, vk::Result>>()?;
            let vert_module =
                create_shader_module_from_file(&device, "src/shaders/triangle-vert.spv")?;
            let frag_module =
                create_shader_module_from_file(&device, "src/shaders/triangle-frag.spv")?;
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
            let vertex_input_state_ci = vk::PipelineVertexInputStateCreateInfo::default();

            let input_assembly_state_ci = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                .primitive_restart_enable(false);
            let viewport = vk::Viewport::builder()
                .x(0.0)
                .y(0.0)
                .width(swapchain_extent.width as f32)
                .height(swapchain_extent.height as f32)
                .min_depth(0.0)
                .max_depth(0.0)
                .build();
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain_extent,
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
            let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::LINE_WIDTH];
            let dynamic_state_ci = vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(&dynamic_states)
                .build();
            let graphics_pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&[])
                .push_constant_ranges(&[])
                .build();
            let graphics_pipeline_layout =
                device.create_pipeline_layout(&graphics_pipeline_layout_ci, None)?;

            let color_attachment_description = vk::AttachmentDescription::builder()
                .format(surface_format.format)
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
                    vk::AccessFlags::COLOR_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                )
                .build();
            let render_pass_ci = vk::RenderPassCreateInfo::builder()
                .attachments(&[color_attachment_description])
                .subpasses(&[subpass_description])
                .dependencies(&[subpass_dependency])
                .build();
            let render_pass = device.create_render_pass(&render_pass_ci, None)?;
            let pipeline_ci = vk::GraphicsPipelineCreateInfo::builder()
                .stages(&shader_stages)
                .vertex_input_state(&vertex_input_state_ci)
                .input_assembly_state(&input_assembly_state_ci)
                .viewport_state(&viewport_state_ci)
                .rasterization_state(&rasterizer_state_ci)
                .multisample_state(&multisample_state_ci)
                .color_blend_state(&color_blend_state_ci)
                .layout(graphics_pipeline_layout)
                .render_pass(render_pass)
                .subpass(0)
                .build();
            let graphics_pipeline = device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_ci], None)
                .map_err(|x| x.1)?[0];
            let mut swapchain_framebuffers = vec![];
            for &swapchain_image_view in &swapchain_image_views {
                let framebuffer_ci = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(&[swapchain_image_view])
                    .width(swapchain_extent.width)
                    .height(swapchain_extent.height)
                    .layers(1)
                    .build();
                let framebuffer = device.create_framebuffer(&framebuffer_ci, None)?;
                swapchain_framebuffers.push(framebuffer);
            }

            let command_pool_ci = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .build();
            let command_pool = device.create_command_pool(&command_pool_ci, None)?;
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(swapchain_framebuffers.len() as u32)
                .build();
            let command_buffers = device.allocate_command_buffers(&command_buffer_alloc_info)?;
            for (index, &command_buffer) in command_buffers.iter().enumerate() {
                let begin_info = vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE)
                    .build();
                device.begin_command_buffer(command_buffer, &begin_info)?;
                let swapchain_framebuffer = swapchain_framebuffers[index];
                let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                    .render_pass(render_pass)
                    .framebuffer(swapchain_framebuffer)
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: swapchain_extent,
                    })
                    .clear_values(&[vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 0.0],
                        },
                    }])
                    .build();
                device.cmd_begin_render_pass(
                    command_buffer,
                    &render_pass_begin_info,
                    vk::SubpassContents::INLINE,
                );
                device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    graphics_pipeline,
                );
                device.cmd_draw(command_buffer, 3, 1, 0, 0);
                device.cmd_end_render_pass(command_buffer);
                device.end_command_buffer(command_buffer)?;
            }

            let semaphore_ci = vk::SemaphoreCreateInfo::default();
            let fence_ci = vk::FenceCreateInfo::builder()
                .flags(vk::FenceCreateFlags::SIGNALED)
                .build();
            let mut image_available_semaphores = vec![];
            let mut render_finished_semaphores = vec![];
            let mut in_flight_fences = vec![];
            for _ in 0..MAX_FRAMES_IN_FLIGHT {
                image_available_semaphores.push(device.create_semaphore(&semaphore_ci, None)?);
                render_finished_semaphores.push(device.create_semaphore(&semaphore_ci, None)?);
                in_flight_fences.push(device.create_fence(&fence_ci, None)?);
            }

            // clean up shader modules
            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);

            Ok(VulkanBase {
                current_frame: 0,
                events_loop,
                entry,
                instance,
                device,
                queue_family_index,
                pdevice,
                window,
                surface_loader,
                surface_format,
                graphics_queue,
                render_pass,
                swapchain_extent,
                swapchain,
                swapchain_handle,
                swapchain_images,
                swapchain_image_views,
                swapchain_framebuffers,
                surface,
                graphics_pipeline_layout,
                graphics_pipeline,
                command_pool,
                command_buffers,
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
                debug_messenger,
            })
        }
    }

    pub unsafe fn draw_frame(&mut self) -> VkResult<()> {
        self.device.wait_for_fences(
            &[self.in_flight_fences[self.current_frame]],
            true,
            std::u64::MAX,
        )?;
        self.device
            .reset_fences(&[self.in_flight_fences[self.current_frame]])?;

        let image_index = self
            .swapchain
            .acquire_next_image_khr(
                self.swapchain_handle,
                std::u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null(),
            )?
            .0;
        let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&[self.image_available_semaphores[self.current_frame]])
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&[self.command_buffers[image_index as usize]])
            .signal_semaphores(&signal_semaphores)
            .build();
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
        self.swapchain
            .queue_present_khr(self.graphics_queue, &present_info)?;
        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        Ok(())
    }

    pub fn start(&mut self) {
        let mut running = true;
        while running {
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
                    _ => (),
                },
                _ => (),
            });
            unsafe {
                let _ = self.draw_frame();
            }
        }
    }
}

impl Drop for VulkanBase {
    fn drop(&mut self) {
        unsafe {
            self.device
                .wait_for_fences(&self.in_flight_fences, true, std::u64::MAX)
                .unwrap();
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device
                    .destroy_semaphore(self.render_finished_semaphores[i], None);
                self.device
                    .destroy_semaphore(self.image_available_semaphores[i], None);
                self.device.destroy_fence(self.in_flight_fences[i], None);
            }

            self.device.destroy_command_pool(self.command_pool, None);
            for &swapchain_framebuffer in &self.swapchain_framebuffers {
                self.device.destroy_framebuffer(swapchain_framebuffer, None);
            }
            self.device.device_wait_idle().unwrap();
            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.graphics_pipeline_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);

            for &image_view in self.swapchain_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.swapchain
                .destroy_swapchain_khr(self.swapchain_handle, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface_khr(self.surface, None);
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
