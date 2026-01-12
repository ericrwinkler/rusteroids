// build.rs
// Build script for Vulkan shader compilation and resource management

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy)]
enum CompilerType {
    Glslc,
    GlslangValidator,
}

fn find_system_compiler() -> (String, CompilerType) {
    // Try glslc first
    if let Ok(output) = Command::new("which").arg("glslc").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return (path, CompilerType::Glslc);
            }
        }
    }
    
    // Try glslangValidator
    if let Ok(output) = Command::new("which").arg("glslangValidator").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return (path, CompilerType::GlslangValidator);
            }
        }
    }
    
    eprintln!("error: No shader compiler found!");
    eprintln!("hint: Install glslang-tools (for glslangValidator) or Vulkan SDK (for glslc)");
    eprintln!("hint: Run: sudo apt install glslang-tools");
    panic!("Shader compiler not found");
}

fn main() {
    // Tell cargo to rerun this build script if any shader files change
    println!("cargo:rerun-if-changed=../../resources/shaders");
    
    // Allow skipping shader compilation with an env var or arg
    let skip_shaders =
        env::var("SKIP_SHADERS").is_ok() || env::args().any(|arg| arg == "--skip-shaders");
    if skip_shaders {
        eprintln!("info: Skipping shader compilation (SKIP_SHADERS set or --skip-shaders arg)");
        return;
    }

    // Try to find a shader compiler
    // Priority: 1. glslc from VULKAN_SDK, 2. system glslc, 3. glslangValidator
    let compiler = if let Ok(vulkan_sdk) = env::var("VULKAN_SDK") {
        println!("cargo:rerun-if-env-changed=VULKAN_SDK");
        let glslc = if cfg!(target_os = "windows") {
            format!("{}\\Bin\\glslc.exe", vulkan_sdk)
        } else {
            format!("{}/bin/glslc", vulkan_sdk)
        };
        if Path::new(&glslc).exists() {
            (glslc, CompilerType::Glslc)
        } else {
            eprintln!("warning: glslc not found in VULKAN_SDK, trying alternatives...");
            find_system_compiler()
        }
    } else {
        println!("cargo:rerun-if-env-changed=VULKAN_SDK");
        eprintln!("info: VULKAN_SDK not set, trying system shader compilers...");
        find_system_compiler()
    };

    let (compiler_path, compiler_type) = compiler;
    eprintln!("info: Using shader compiler: {:?} ({:?})", compiler_path, compiler_type);

    let shader_dir = PathBuf::from("../../resources/shaders");
    let target_dir = PathBuf::from("../../target/shaders");
    
    // Create target directory
    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        eprintln!("warning: Failed to create target directory: {}", e);
        return;
    }

    // Process shader files recursively
    fn compile_shaders_recursive(
        shader_dir: &Path,
        target_dir: &Path,
        compiler_path: &str,
        compiler_type: CompilerType,
        compiled_count: &mut i32
    ) {
        let shader_files = match std::fs::read_dir(shader_dir) {
            Ok(files) => files,
            Err(_) => return,
        };

        for entry in shader_files {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            
            // Recurse into subdirectories
            if path.is_dir() {
                compile_shaders_recursive(&path, target_dir, compiler_path, compiler_type, compiled_count);
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
                        let status = match compiler_type {
                            CompilerType::Glslc => {
                                Command::new(compiler_path)
                                    .arg("-I")
                                    .arg("../../resources/shaders") // Allow includes from shader root
                                    .arg(&path)
                                    .arg("-o")
                                    .arg(&out_file)
                                    .status()
                            }
                            CompilerType::GlslangValidator => {
                                Command::new(compiler_path)
                                    .arg("-V")  // Vulkan semantics
                                    .arg("-I../../resources/shaders")
                                    .arg(&path)
                                    .arg("-o")
                                    .arg(&out_file)
                                    .status()
                            }
                        };

                        match status {
                            Ok(s) if s.success() => {
                                eprintln!("info: Compiled {:?} -> {:?}", path.file_name().unwrap(), out_file.file_name().unwrap());
                                *compiled_count += 1;
                            }
                            Ok(s) => {
                                eprintln!("error: Shader compilation failed for {:?} with exit code: {}", path, s.code().unwrap_or(-1));
                                panic!("Shader compilation failed");
                            }
                            Err(e) => {
                                eprintln!("error: Failed to run shader compiler for {:?}: {}", path, e);
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
    
    let mut compiled_count = 0;
    compile_shaders_recursive(&shader_dir, &target_dir, &compiler_path, compiler_type, &mut compiled_count);

    if compiled_count > 0 {
        eprintln!("info: Successfully compiled {} shader(s)", compiled_count);
    } else {
        eprintln!("info: All shaders are up to date");
    }
}
