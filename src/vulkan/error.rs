use crate::freetype;
use ash::vk;
use failure_derive::Fail;
use image;

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
    #[fail(display = "{}", _0)]
    FtError(#[cause] freetype::FtError),
}

impl From<image::ImageError> for VulkanError {
    fn from(err: image::ImageError) -> VulkanError {
        VulkanError::ImageError(err)
    }
}

impl From<std::ffi::NulError> for VulkanError {
    fn from(err: std::ffi::NulError) -> VulkanError {
        VulkanError::NulError(err)
    }
}

impl From<vk::Result> for VulkanError {
    fn from(err: vk::Result) -> VulkanError {
        VulkanError::VkError(err)
    }
}

impl From<std::io::Error> for VulkanError {
    fn from(err: std::io::Error) -> VulkanError {
        VulkanError::Io(err)
    }
}

impl From<freetype::FtError> for VulkanError {
    fn from(err: freetype::FtError) -> VulkanError {
        VulkanError::FtError(err)
    }
}

pub type VulkanResult<T> = Result<T, VulkanError>;
