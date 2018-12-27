pub mod app;
mod buffer;
pub mod error;
mod one_time_command_buffer;
mod texture;

pub use self::app::VulkanApp;

use self::error::{from_vk_result, VulkanError, VulkanResult};
use ash::{
    extensions::{DebugUtils, Surface, Swapchain, Win32Surface},
    prelude::VkResult,
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk, vk_make_version, Entry, Instance,
};
use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_void},
    rc::Rc,
};
use winit::dpi::LogicalSize;

pub struct VulkanCore {
    entry: ash::Entry,
    pub instance: ash::Instance,
    pub device: Rc<ash::Device>,
    pub physical_device: vk::PhysicalDevice,
    pub swapchain: Swapchain,
    pub graphics_queue: vk::Queue,
    pub graphics_queue_family_index: u32,
    pub transfer_queue: vk::Queue,
    pub transfer_queue_family_index: u32,

    pub surface: Surface,
    pub surface_handle: vk::SurfaceKHR,

    pub events_loop: winit::EventsLoop,
    pub window: winit::Window,

    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl VulkanCore {
    pub fn new(name: &str, screen_width: u32, screen_height: u32) -> VulkanResult<VulkanCore> {
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
        let app_name = CString::new(name)?;
        let entry = Entry::new().unwrap();
        let layer_names = [CString::new("VK_LAYER_LUNARG_standard_validation")?];
        let layer_names_raw: Vec<*const c_char> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();
        let extension_names_raw = vec![
            Surface::name().as_ptr(),
            Win32Surface::name().as_ptr(),
            DebugUtils::name().as_ptr(),
        ];
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&app_name)
            .engine_version(0)
            .api_version(vk_make_version!(1, 1, 0))
            .build();
        let instance_ci = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layer_names_raw)
            .enabled_extension_names(&extension_names_raw)
            .build();
        unsafe {
            let instance = entry
                .create_instance(&instance_ci, None)
                .expect("Instance creation error");
            let debug_messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                )
                .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
                .pfn_user_callback(Some(vulkan_debug_callback))
                .build();
            let mut debug_messenger: vk::DebugUtilsMessengerEXT = Default::default();
            create_debug_utils_messenger_ext(
                &entry,
                instance.handle(),
                &debug_messenger_create_info,
                std::ptr::null(),
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
                        "cannot find suitable queue family".to_owned(),
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

            Ok(VulkanCore {
                entry,
                instance,
                device: Rc::new(device),
                physical_device,

                swapchain,
                graphics_queue,
                graphics_queue_family_index,
                transfer_queue,
                transfer_queue_family_index,

                surface,
                surface_handle,
                events_loop,
                window,

                debug_messenger,
            })
        }
    }

    pub unsafe fn find_memory_type(
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

    pub unsafe fn create_buffer(
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
}

impl Drop for VulkanCore {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            self.surface.destroy_surface_khr(self.surface_handle, None);
            destroy_debug_utils_messenger_ext(
                &self.entry,
                self.instance.handle(),
                self.debug_messenger,
                std::ptr::null(),
            )
            .unwrap();
            self.instance.destroy_instance(None);
        }
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

#[cfg(target_os = "windows")]
unsafe fn create_surface<E: EntryV1_0, I: InstanceV1_0>(
    entry: &E,
    instance: &I,
    window: &winit::Window,
) -> Result<vk::SurfaceKHR, vk::Result> {
    use winapi::shared::windef::HWND;
    use winapi::um::libloaderapi::GetModuleHandleW;
    use winit::os::windows::WindowExt;

    let hwnd = window.get_hwnd() as HWND;
    let hinstance = GetModuleHandleW(std::ptr::null()) as *const c_void;
    let win32_create_info = vk::Win32SurfaceCreateInfoKHR {
        s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
        p_next: std::ptr::null(),
        flags: Default::default(),
        hinstance,
        hwnd: hwnd as *const c_void,
    };
    let win32_surface = Win32Surface::new(entry, instance);
    win32_surface.create_win32_surface_khr(&win32_create_info, None)
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
