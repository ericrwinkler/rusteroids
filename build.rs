// build.rs
// Build script for Vulkan shader compilation and resource management

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Recursively compile all shader files in a directory
fn compile_shaders_recursive(
    shader_dir: &Path,
    target_dir: &Path,
    glslc: &str,
    compiled_count: &mut i32
) {
    let shader_files = match std::fs::read_dir(shader_dir) {
        Ok(files) => files,
        Err(_) => {
            eprintln!("info: No shader directory found at: {:?}", shader_dir);
            return;
        }
    };

    for entry in shader_files {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("warning: Error reading shader directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();
        
        // Recurse into subdirectories
        if path.is_dir() {
            compile_shaders_recursive(&path, target_dir, glslc, compiled_count);
            continue;
        }
        
        // Skip include files (.glsl) - they're not standalone shaders
        if let Some(ext) = path.extension() {
            if ext == "glsl" {
                continue;
            }
            
            if ext == "vert" || ext == "frag" || ext == "comp" || ext == "geom" || ext == "tesc" || ext == "tese" {
                let out_file = target_dir
                    .join(path.file_stem().unwrap())
                    .with_extension("spv");

                // Check if recompilation is needed
                let needs_compile = if let (Ok(src_meta), Ok(dst_meta)) = (
                    std::fs::metadata(&path),
                    std::fs::metadata(&out_file)
                ) {
                    src_meta.modified().unwrap() > dst_meta.modified().unwrap()
                } else {
                    true // Compile if either file doesn't exist or we can't get metadata
                };

                if needs_compile {
                    // Add -I flag to support #include directives
                    let status = Command::new(glslc)
                        .arg("-I")
                        .arg("resources/shaders") // Allow includes from shader root
                        .arg(&path)
                        .arg("-o")
                        .arg(&out_file)
                        .status();

                    match status {
                        Ok(s) if s.success() => {
                            eprintln!("info: Compiled {:?} -> {:?}", path.file_name().unwrap(), out_file.file_name().unwrap());
                            *compiled_count += 1;
                        }
                        Ok(s) => {
                            eprintln!("error: glslc failed for {:?} with exit code: {}", path, s.code().unwrap_or(-1));
                            panic!("Shader compilation failed");
                        }
                        Err(e) => {
                            eprintln!("error: Failed to run glslc for {:?}: {}", path, e);
                            panic!("Failed to execute shader compiler");
                        }
                    }
                } else {
                    eprintln!("info: Shader {:?} is up to date", path.file_name().unwrap());
                }
            }
        }
    }
}

fn main() {
    // Tell cargo to rerun this build script if any shader files change
    println!("cargo:rerun-if-changed=resources/shaders");
    
    eprintln!("[BUILD] Starting shader compilation...");
    
    // Allow skipping shader compilation with an env var or arg
    let skip_shaders =
        env::var("SKIP_SHADERS").is_ok() || env::args().any(|arg| arg == "--skip-shaders");
    if skip_shaders {
        eprintln!("info: Skipping shader compilation (SKIP_SHADERS set or --skip-shaders arg)");
        return;
    }

    // Check for Vulkan SDK
    let vulkan_sdk = match env::var("VULKAN_SDK") {
        Ok(sdk) => sdk,
        Err(_) => {
            println!("cargo:rerun-if-env-changed=VULKAN_SDK");
            eprintln!("warning: VULKAN_SDK not set, shader compilation skipped");
            eprintln!("hint: Install Vulkan SDK and set VULKAN_SDK environment variable");
            return;
        }
    };

    let glslc = if cfg!(target_os = "windows") {
        format!("{}\\Bin\\glslc.exe", vulkan_sdk)
    } else {
        format!("{}/bin/glslc", vulkan_sdk)
    };

    // Verify glslc exists
    if !Path::new(&glslc).exists() {
        eprintln!("error: glslc not found at: {}", glslc);
        eprintln!("hint: Ensure Vulkan SDK is properly installed");
        panic!("Shader compiler not found");
    }

    let shader_dir = PathBuf::from("resources/shaders");
    let target_dir = PathBuf::from("target/shaders");
    
    // Create target directory
    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        eprintln!("warning: Failed to create target directory: {}", e);
        return;
    }

    // Process shader files recursively
    let mut compiled_count = 0;
    compile_shaders_recursive(&shader_dir, &target_dir, &glslc, &mut compiled_count);

    if compiled_count > 0 {
        eprintln!("info: Successfully compiled {} shader(s)", compiled_count);
    } else {
        eprintln!("info: All shaders are up to date");
    }
}
