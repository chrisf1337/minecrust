use std::process::Command;

fn main() {
    // triangle
    if !Command::new("D:/VulkanSDK/1.1.85.0/Bin/glslangValidator.exe")
        .args(&[
            "-V",
            "src/shaders/triangle.vert",
            "-o",
            "src/shaders/triangle-vert.spv",
        ])
        .status()
        .unwrap()
        .success()
    {
        panic!("failed to compile triangle.vert");
    }
    if !Command::new("D:/VulkanSDK/1.1.85.0/Bin/glslangValidator.exe")
        .args(&[
            "-V",
            "src/shaders/triangle.frag",
            "-o",
            "src/shaders/triangle-frag.spv",
        ])
        .status()
        .unwrap()
        .success()
    {
        panic!("failed to compile triangle.frag");
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
}
