use std::path::Path;
use std::process::Command;

fn main() {
    Command::new("D:/VulkanSDK/1.1.85.0/Bin/glslangValidator.exe")
        .args(&[
            "-V",
            "src/shaders/triangle.vert",
            "-o",
            "src/shaders/triangle-vert.spv",
        ])
        .status()
        .unwrap();
    Command::new("D:/VulkanSDK/1.1.85.0/Bin/glslangValidator.exe")
        .args(&[
            "-V",
            "src/shaders/triangle.frag",
            "-o",
            "src/shaders/triangle-frag.spv",
        ])
        .status()
        .unwrap();
}
