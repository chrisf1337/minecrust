use ash;
use ash::{
    extensions::{DebugReport, DebugUtils, Surface, Swapchain, Win32Surface},
    version::{DeviceV1_0, DeviceV1_1, EntryV1_0, EntryV1_1, InstanceV1_0, InstanceV1_1},
    vk, vk_make_version, Device, Entry, Instance,
};
use std::{
    ffi::{CStr, CString},
    os::raw::*,
    ptr,
};
#[cfg(target_os = "windows")]
use winapi;
use winit;
use winit::dpi::LogicalSize;

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    p_user_data: *mut c_void,
) -> u32 {
    println!("{:?}", CStr::from_ptr((*p_callback_data).p_message));
    vk::FALSE
}

unsafe fn create_debug_utils_messenger_ext(
    entry: &Entry,
    instance: &vk::Instance,
    create_info: &vk::DebugUtilsMessengerCreateInfoEXT,
    callback: *const vk::PFN_vkCreateDebugUtilsMessengerEXT,
) -> Result<(), vk::Result> {
    if let Some(func) = entry.get_instance_proc_addr(
        instance.clone(),
        CString::new("vkCreateDebugUtilsMessengerEXT")
            .unwrap()
            .as_ptr(),
    ) {
        let func: vk::PFN_vkCreateDebugUtilsMessengerEXT = std::mem::transmute(func);
        Ok(func(instance, create_info, ptr::null(), callback))
    } else {
        Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT)
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

fn find_memorytype_index_f<F: Fn(vk::MemoryPropertyFlags, vk::MemoryPropertyFlags) -> bool>(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
    f: F,
) -> Option<u32> {
    let mut memory_type_bits = memory_req.memory_type_bits;
    for (index, ref memory_type) in memory_prop.memory_types.iter().enumerate() {
        if memory_type_bits & 1 == 1 && f(memory_type.property_flags, flags) {
            return Some(index as u32);
        }
        memory_type_bits >>= 1;
    }
    None
}

fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    let best_suitable_index =
        find_memorytype_index_f(memory_req, memory_prop, flags, |property_flags, flags| {
            property_flags == flags
        });
    if best_suitable_index.is_some() {
        return best_suitable_index;
    }
    find_memorytype_index_f(memory_req, memory_prop, flags, |property_flags, flags| {
        property_flags & flags == flags
    })
}

fn record_submit_command_buffer<D: DeviceV1_1, F: FnOnce(&D, vk::CommandBuffer)>(
    device: &D,
    command_buffer: vk::CommandBuffer,
    submit_queue: vk::Queue,
    wait_mask: &[vk::PipelineStageFlags],
    wait_semaphores: &[vk::Semaphore],
    signal_semaphores: &[vk::Semaphore],
    f: F,
) -> ash::prelude::VkResult<()> {
    unsafe {
        device.reset_command_buffer(
            command_buffer,
            vk::CommandBufferResetFlags::RELEASE_RESOURCES,
        )?;
        let command_buffer_begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: ptr::null(),
            p_inheritance_info: ptr::null(),
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
        };
        device.begin_command_buffer(command_buffer, &command_buffer_begin_info)?;
        f(device, command_buffer);
        device.end_command_buffer(command_buffer)?;
        let fence_create_info = vk::FenceCreateInfo {
            s_type: vk::StructureType::FENCE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::FenceCreateFlags::empty(),
        };
        let submit_fence = device.create_fence(&fence_create_info, None)?;
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: ptr::null(),
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_mask.as_ptr(),
            command_buffer_count: 1,
            p_command_buffers: &command_buffer,
            signal_semaphore_count: signal_semaphores.len() as u32,
            p_signal_semaphores: signal_semaphores.as_ptr(),
        };
        device.queue_submit(submit_queue, &[submit_info], submit_fence)?;
        device.wait_for_fences(&[submit_fence], true, std::u64::MAX)?;
        device.destroy_fence(submit_fence, None);
    }
    Ok(())
}

pub struct VulkanBase {
    pub entry: Entry,
    pub instance: Instance,
    pub device: Device,
    pub surface_loader: Surface,
    pub swapchain_loader: Swapchain,
    pub window: winit::Window,
    pub events_loop: winit::EventsLoop,

    pub pdevice: vk::PhysicalDevice,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub queue_family_index: u32,
    pub present_queue: vk::Queue,

    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,

    pub swapchain: vk::SwapchainKHR,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,

    pub pool: vk::CommandPool,
    pub draw_command_buffer: vk::CommandBuffer,
    pub setup_command_buffer: vk::CommandBuffer,

    pub depth_image: vk::Image,
    pub depth_image_view: vk::ImageView,
    pub depth_image_memory: vk::DeviceMemory,

    pub present_complete_semaphore: vk::Semaphore,
    pub rendering_complete_semaphore: vk::Semaphore,
    // debug
    // pub debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl VulkanBase {
    pub fn new(screen_width: u32, screen_height: u32) -> Result<VulkanBase, vk::Result> {
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
            // let debug_info = vk::DebugReportCallbackCreateInfoEXT {
            //     s_type: vk::StructureType::DEBUG_REPORT_CALLBACK_CREATE_INFO_EXT,
            //     p_next: ptr::null(),
            //     flags: vk::DebugReportFlagsEXT::ERROR
            //         | vk::DebugReportFlagsEXT::WARNING
            //         | vk::DebugReportFlagsEXT::PERFORMANCE_WARNING,
            //     pfn_callback: Some(vulkan_debug_callback),
            //     p_user_data: ptr::null_mut(),
            // };
            // let debug_report_loader = DebugReport::new(&entry, &instance);
            // let debug_call_back =
            //     debug_report_loader.create_debug_report_callback_ext(&debug_info, None)?;
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
                                Some((*pdevice, index))
                            } else {
                                None
                            }
                        })
                        .nth(0)
                })
                .filter_map(|v| v)
                .nth(0)
                .expect("No physical device");
            let queue_family_index = queue_family_index as u32;
            let device_extension_names_raw = [Swapchain::name().as_ptr()];
            let features = vk::PhysicalDeviceFeatures {
                shader_clip_distance: 1,
                ..Default::default()
            };
            let priorities = [1.0];
            let queue_info = vk::DeviceQueueCreateInfo {
                s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
                p_next: ptr::null(),
                flags: Default::default(),
                queue_family_index,
                p_queue_priorities: priorities.as_ptr(),
                queue_count: priorities.len() as u32,
            };
            let device_create_info = vk::DeviceCreateInfo {
                s_type: vk::StructureType::DEVICE_CREATE_INFO,
                p_next: ptr::null(),
                flags: Default::default(),
                queue_create_info_count: 1,
                p_queue_create_infos: &queue_info,
                enabled_layer_count: 0,
                pp_enabled_layer_names: ptr::null(),
                enabled_extension_count: device_extension_names_raw.len() as u32,
                pp_enabled_extension_names: device_extension_names_raw.as_ptr(),
                p_enabled_features: &features,
            };
            let device = instance
                .create_device(pdevice, &device_create_info, None)
                .unwrap();
            let present_queue = device.get_device_queue(queue_family_index, 0);
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
            let surface_resolution = if surface_capabilities.current_extent.width == std::u32::MAX {
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
            let swapchain_loader = Swapchain::new(&instance, &device);
            let swapchain_create_info = vk::SwapchainCreateInfoKHR {
                s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
                p_next: ptr::null(),
                flags: Default::default(),
                surface,
                min_image_count: desired_image_count,
                image_color_space: surface_format.color_space,
                image_format: surface_format.format,
                image_extent: surface_resolution,
                image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                image_sharing_mode: vk::SharingMode::EXCLUSIVE,
                pre_transform,
                composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
                present_mode,
                clipped: 1,
                old_swapchain: vk::SwapchainKHR::null(),
                image_array_layers: 1,
                p_queue_family_indices: ptr::null(),
                queue_family_index_count: 0,
            };
            let swapchain = swapchain_loader.create_swapchain_khr(&swapchain_create_info, None)?;
            let pool_create_info = vk::CommandPoolCreateInfo {
                s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                queue_family_index,
            };
            let pool = device.create_command_pool(&pool_create_info, None)?;
            let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
                s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
                p_next: ptr::null(),
                command_buffer_count: 2,
                command_pool: pool,
                level: vk::CommandBufferLevel::PRIMARY,
            };
            let command_buffers = device.allocate_command_buffers(&command_buffer_allocate_info)?;
            let setup_command_buffer = command_buffers[0];
            let draw_command_buffer = command_buffers[1];
            let present_images = swapchain_loader.get_swapchain_images_khr(swapchain)?;
            let present_image_views: Vec<vk::ImageView> = present_images
                .iter()
                .map(|&image| {
                    let create_view_info = vk::ImageViewCreateInfo {
                        s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                        p_next: ptr::null(),
                        flags: Default::default(),
                        view_type: vk::ImageViewType::TYPE_2D,
                        format: surface_format.format,
                        components: vk::ComponentMapping {
                            r: vk::ComponentSwizzle::R,
                            g: vk::ComponentSwizzle::G,
                            b: vk::ComponentSwizzle::B,
                            a: vk::ComponentSwizzle::A,
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
                    device.create_image_view(&create_view_info, None).unwrap()
                })
                .collect();
            let device_memory_properties = instance.get_physical_device_memory_properties(pdevice);
            let depth_image_create_info = vk::ImageCreateInfo {
                s_type: vk::StructureType::IMAGE_CREATE_INFO,
                p_next: ptr::null(),
                flags: Default::default(),
                image_type: vk::ImageType::TYPE_2D,
                format: vk::Format::D16_UNORM,
                extent: vk::Extent3D {
                    width: surface_resolution.width,
                    height: surface_resolution.height,
                    depth: 1,
                },
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::OPTIMAL,
                usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                queue_family_index_count: 0,
                p_queue_family_indices: ptr::null(),
                initial_layout: vk::ImageLayout::UNDEFINED,
            };
            let depth_image = device.create_image(&depth_image_create_info, None)?;
            let depth_image_memory_req = device.get_image_memory_requirements(depth_image);
            let depth_image_memory_index = find_memorytype_index(
                &depth_image_memory_req,
                &device_memory_properties,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .expect("Unable to find suitable memory index for depth image");
            let depth_image_allocate_info = vk::MemoryAllocateInfo {
                s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
                p_next: ptr::null(),
                allocation_size: depth_image_memory_req.size,
                memory_type_index: depth_image_memory_index,
            };
            let depth_image_memory = device.allocate_memory(&depth_image_allocate_info, None)?;
            device.bind_image_memory(depth_image, depth_image_memory, 0)?;
            record_submit_command_buffer(
                &device,
                setup_command_buffer,
                present_queue,
                &[vk::PipelineStageFlags::BOTTOM_OF_PIPE],
                &[],
                &[],
                |device, setup_command_buffer| {
                    let layout_transition_barrier = vk::ImageMemoryBarrier {
                        s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                        p_next: ptr::null(),
                        src_access_mask: Default::default(),
                        dst_access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                            | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        old_layout: vk::ImageLayout::UNDEFINED,
                        new_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        image: depth_image,
                        subresource_range: vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::DEPTH,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        },
                    };
                    device.cmd_pipeline_barrier(
                        setup_command_buffer,
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                        vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[layout_transition_barrier],
                    );
                },
            )?;
            let depth_image_view_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                p_next: ptr::null(),
                flags: Default::default(),
                view_type: vk::ImageViewType::TYPE_2D,
                format: depth_image_create_info.format,
                components: vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                },
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::DEPTH,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image: depth_image,
            };
            let depth_image_view = device.create_image_view(&depth_image_view_info, None)?;
            let semaphore_create_info = vk::SemaphoreCreateInfo {
                s_type: vk::StructureType::SEMAPHORE_CREATE_INFO,
                p_next: ptr::null(),
                flags: Default::default(),
            };
            let present_complete_semaphore =
                device.create_semaphore(&semaphore_create_info, None)?;
            let rendering_complete_semaphore =
                device.create_semaphore(&semaphore_create_info, None)?;

            let renderpass_attachments = [
                vk::AttachmentDescription {
                    format: surface_format.format,
                    flags: vk::AttachmentDescriptionFlags::empty(),
                    samples: vk::SampleCountFlags::TYPE_1,
                    load_op: vk::AttachmentLoadOp::CLEAR,
                    store_op: vk::AttachmentStoreOp::STORE,
                    stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                    stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                    initial_layout: vk::ImageLayout::UNDEFINED,
                    final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                },
                vk::AttachmentDescription {
                    format: vk::Format::D16_UNORM,
                    flags: vk::AttachmentDescriptionFlags::empty(),
                    samples: vk::SampleCountFlags::TYPE_1,
                    load_op: vk::AttachmentLoadOp::CLEAR,
                    store_op: vk::AttachmentStoreOp::DONT_CARE,
                    stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                    stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                    initial_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                },
            ];
            let color_attachment_ref = vk::AttachmentReference {
                attachment: 0,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            };
            let depth_attachment_ref = vk::AttachmentReference {
                attachment: 1,
                layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            };
            let dependency = vk::SubpassDependency {
                dependency_flags: Default::default(),
                src_subpass: vk::SUBPASS_EXTERNAL,
                dst_subpass: Default::default(),
                src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                src_access_mask: Default::default(),
                dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                    | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            };
            let subpass = vk::SubpassDescription {
                color_attachment_count: 1,
                p_color_attachments: &color_attachment_ref,
                p_depth_stencil_attachment: &depth_attachment_ref,
                flags: Default::default(),
                pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
                input_attachment_count: 0,
                p_input_attachments: ptr::null(),
                p_resolve_attachments: ptr::null(),
                preserve_attachment_count: 0,
                p_preserve_attachments: ptr::null(),
            };
            let render_pass_create_info = vk::RenderPassCreateInfo {
                s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
                flags: Default::default(),
                p_next: ptr::null(),
                attachment_count: renderpass_attachments.len() as u32,
                p_attachments: renderpass_attachments.as_ptr(),
                subpass_count: 1,
                p_subpasses: &subpass,
                dependency_count: 1,
                p_dependencies: &dependency,
            };
            let render_pass = device.create_render_pass(&render_pass_create_info, None)?;
            let framebuffers: Vec<vk::Framebuffer> = present_image_views
                .iter()
                .map(|&present_image_view| {
                    let framebufffer_attachments = [present_image_view, depth_image_view];
                    let framebuffer_create_info = vk::FramebufferCreateInfo {
                        s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
                        p_next: ptr::null(),
                        flags: Default::default(),
                        render_pass,
                        attachment_count: framebufffer_attachments.len() as u32,
                        p_attachments: framebufffer_attachments.as_ptr(),
                        width: surface_resolution.width,
                        height: surface_resolution.height,
                        layers: 1,
                    };
                    device
                        .create_framebuffer(&framebuffer_create_info, None)
                        .unwrap()
                })
                .collect();
            let index_buffer_data = [0u32, 1, 2];

            Ok(VulkanBase {
                events_loop,
                entry,
                instance,
                device,
                queue_family_index,
                pdevice,
                device_memory_properties,
                window,
                surface_loader,
                surface_format,
                present_queue,
                surface_resolution,
                swapchain_loader,
                swapchain,
                present_images,
                present_image_views,
                pool,
                draw_command_buffer,
                setup_command_buffer,
                depth_image,
                depth_image_view,
                present_complete_semaphore,
                rendering_complete_semaphore,
                surface,
                depth_image_memory,
                // debug_messenger,
            })
        }
    }
}

impl Drop for VulkanBase {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device
                .destroy_semaphore(self.present_complete_semaphore, None);
            self.device
                .destroy_semaphore(self.rendering_complete_semaphore, None);
            self.device.free_memory(self.depth_image_memory, None);
            self.device.destroy_image_view(self.depth_image_view, None);
            self.device.destroy_image(self.depth_image, None);
            for &image_view in self.present_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.device.destroy_command_pool(self.pool, None);
            self.swapchain_loader
                .destroy_swapchain_khr(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface_khr(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}
