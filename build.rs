// build.rs
// All build scripts for the Vulkan triangle project should be placed here.

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Allow skipping shader compilation with an env var or arg
    let skip_shaders =
        env::var("SKIP_SHADERS").is_ok() || env::args().any(|arg| arg == "--skip-shaders");
    if skip_shaders {
        println!("cargo:rerun-if-changed=shaders");
        eprintln!("info: Skipping shader compilation (SKIP_SHADERS set or --skip-shaders arg)");
        return;
    }

    let vulkan_bin = if let Ok(sdk) = env::var("VULKAN_SDK") {
        format!("{}\\Bin", sdk)
    } else {
        println!("cargo:rerun-if-env-changed=VULKAN_SDK");
        eprintln!("info: VULKAN_SDK not set, shader compilation skipped");
        return;
    };
    let glslc = format!("{}\\glslc.exe", vulkan_bin);
    let shader_dir = "resources/shaders";
    let target_dir = "target/shaders";
    std::fs::create_dir_all(target_dir).ok();
    let shader_files =
        std::fs::read_dir(shader_dir).unwrap_or_else(|_| panic!("No {} directory", shader_dir));
    for entry in shader_files {
        let entry = entry.unwrap();
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "vert" || ext == "frag" {
                let out_file = Path::new(target_dir)
                    .join(path.file_stem().unwrap())
                    .with_extension("spv");
                let status = Command::new(&glslc)
                    .arg(&path)
                    .arg("-o")
                    .arg(&out_file)
                    .status();
                match status {
                    Ok(s) if s.success() => {
                        eprintln!("info: Compiled {:?} -> {:?}", path, out_file)
                    }
                    Ok(s) => panic!("glslc failed for {:?} with status {}", path, s),
                    Err(e) => panic!("Failed to run glslc for {:?}: {}", path, e),
                }
            }
        }
    }
}
