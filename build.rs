use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let vulkan_bin = r"C:\VulkanSDK\1.4.313.2\Bin";
    let glslc = format!(r"{}\glslc.exe", vulkan_bin);
    let shader_dir = "shaders";
    let target_dir = "target/shaders";

    // Create the target directory if it doesn't exist
    if !Path::new(target_dir).exists() {
        fs::create_dir_all(target_dir).expect("Failed to create target/shaders directory");
    }

    // Compile all .vert and .frag files in the shaders directory
    let shader_files = fs::read_dir(shader_dir).expect("Failed to read shaders directory");
    for entry in shader_files {
        let entry = entry.expect("Failed to read shader file entry");
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "vert" || ext == "frag" {
                let file_name = path.file_stem().unwrap().to_string_lossy();
                let out_path = format!("{}/{}.spv", target_dir, file_name);
                let status = Command::new(&glslc)
                    .arg(&path)
                    .arg("-o")
                    .arg(&out_path)
                    .status()
                    .expect("Failed to run glslc");
                if !status.success() {
                    panic!("Failed to compile shader: {:?}", path);
                }
            }
        }
    }
}
