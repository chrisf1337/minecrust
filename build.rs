use std::process::Command;

fn main() {
    // graphics
    if !Command::new("D:/VulkanSDK/1.1.85.0/Bin/glslangValidator.exe")
        .args(&[
            "-V",
            "src/shaders/graphics.vert",
            "-o",
            "src/shaders/graphics-vert.spv",
        ])
        .status()
        .unwrap()
        .success()
    {
        panic!("failed to compile graphics.vert");
    }
    if !Command::new("D:/VulkanSDK/1.1.85.0/Bin/glslangValidator.exe")
        .args(&[
            "-V",
            "src/shaders/graphics.frag",
            "-o",
            "src/shaders/graphics-frag.spv",
        ])
        .status()
        .unwrap()
        .success()
    {
        panic!("failed to compile graphics.frag");
    }

    // text
    if !Command::new("D:/VulkanSDK/1.1.85.0/Bin/glslangValidator.exe")
        .args(&[
            "-V",
            "src/shaders/text.vert",
            "-o",
            "src/shaders/text-vert.spv",
        ])
        .status()
        .unwrap()
        .success()
    {
        panic!("failed to compile text.vert");
    }
    if !Command::new("D:/VulkanSDK/1.1.85.0/Bin/glslangValidator.exe")
        .args(&[
            "-V",
            "src/shaders/text.frag",
            "-o",
            "src/shaders/text-frag.spv",
        ])
        .status()
        .unwrap()
        .success()
    {
        panic!("failed to compile text.frag");
    }

    // selection
    if !Command::new("D:/VulkanSDK/1.1.85.0/Bin/glslangValidator.exe")
        .args(&[
            "-V",
            "src/shaders/selection.vert",
            "-o",
            "src/shaders/selection-vert.spv",
        ])
        .status()
        .unwrap()
        .success()
    {
        panic!("failed to compile selection.vert");
    }
    if !Command::new("D:/VulkanSDK/1.1.85.0/Bin/glslangValidator.exe")
        .args(&[
            "-V",
            "src/shaders/selection.frag",
            "-o",
            "src/shaders/selection-frag.spv",
        ])
        .status()
        .unwrap()
        .success()
    {
        panic!("failed to compile selection.frag");
    }

    // crosshair
    if !Command::new("D:/VulkanSDK/1.1.85.0/Bin/glslangValidator.exe")
        .args(&[
            "-V",
            "src/shaders/crosshair.vert",
            "-o",
            "src/shaders/crosshair-vert.spv",
        ])
        .status()
        .unwrap()
        .success()
    {
        panic!("failed to compile crosshair.vert");
    }
    if !Command::new("D:/VulkanSDK/1.1.85.0/Bin/glslangValidator.exe")
        .args(&[
            "-V",
            "src/shaders/crosshair.frag",
            "-o",
            "src/shaders/crosshair-frag.spv",
        ])
        .status()
        .unwrap()
        .success()
    {
        panic!("failed to compile crosshair.frag");
    }
}
