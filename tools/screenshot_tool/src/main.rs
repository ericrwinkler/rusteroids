use anyhow::{Context, Result};
use clap::{Arg, Command};
use image::{ImageBuffer, RgbaImage};
use std::path::PathBuf;
use std::process::{Command as ProcessCommand};
use std::thread;
use std::time::Duration;

mod analysis;
use analysis::{analyze_screenshot, ScreenshotAnalysis};

#[cfg(windows)]
use winapi::um::winuser::{GetWindowRect, IsWindowVisible, EnumWindows, GetWindowThreadProcessId};
#[cfg(windows)]
use winapi::shared::windef::RECT;

const DEFAULT_SCREENSHOT_DIR: &str = "screenshots";
const DEFAULT_WAIT_TIME_MS: u64 = 3000;

#[derive(Debug)]
struct ScreenshotConfig {
    output_dir: PathBuf,
    wait_time: Duration,
    window_title: Option<String>,
    teapot_executable: PathBuf,
    auto_close: bool,
    filename_prefix: String,
    clean_before_capture: bool,
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from(DEFAULT_SCREENSHOT_DIR),
            wait_time: Duration::from_millis(DEFAULT_WAIT_TIME_MS),
            window_title: None,
            teapot_executable: PathBuf::from("../../target/debug/teapot_app.exe"),
            auto_close: true,
            filename_prefix: "teapot".to_string(),
            clean_before_capture: true,
        }
    }
}

fn main() -> Result<()> {
    let matches = Command::new("screenshot_tool")
        .about("Captures screenshots of the teapot rendering application for automated testing")
        .arg(
            Arg::new("analyze")
                .long("analyze")
                .value_name("FILE")
                .help("Analyze an existing screenshot file")
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("DIR")
                .help("Output directory for screenshots")
                .default_value(DEFAULT_SCREENSHOT_DIR),
        )
        .arg(
            Arg::new("wait")
                .short('w')
                .long("wait")
                .value_name("MILLISECONDS")
                .help("Time to wait before taking screenshot")
                .default_value("3000"),
        )
        .arg(
            Arg::new("window-title")
                .short('t')
                .long("window-title")
                .value_name("TITLE")
                .help("Window title to search for (optional)")
        )
        .arg(
            Arg::new("executable")
                .short('e')
                .long("executable")
                .value_name("PATH")
                .help("Path to teapot executable")
                .default_value("../../target/debug/teapot_app.exe"),
        )
        .arg(
            Arg::new("keep-open")
                .long("keep-open")
                .help("Keep the application running after screenshot")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-clean")
                .long("no-clean")
                .help("Don't delete existing screenshots before capturing")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("prefix")
                .short('p')
                .long("prefix")
                .value_name("PREFIX")
                .help("Filename prefix for screenshots")
                .default_value("teapot"),
        )
        .get_matches();

    // Check if we're analyzing an existing file
    if let Some(analyze_file) = matches.get_one::<String>("analyze") {
        let screenshot_path = PathBuf::from(analyze_file);
        println!("Analyzing screenshot: {:?}", screenshot_path);
        
        if !screenshot_path.exists() {
            return Err(anyhow::anyhow!("Screenshot file does not exist: {:?}", screenshot_path));
        }
        
        let result = analyze_screenshot(&screenshot_path)?;
        println!("Analysis Result: {:?}", result);
        println!("Content Classification: {:?}", result.likely_content);
        println!("Colored Pixels: {:.1}%", result.colored_ratio * 100.0);
        println!("Average Brightness: {}/255", result.avg_brightness);
        println!("Black Pixels: {:.1}%", result.black_ratio * 100.0);
        println!("White Pixels: {:.1}%", result.white_ratio * 100.0);
        return Ok(());
    }

    let config = ScreenshotConfig {
        output_dir: PathBuf::from(matches.get_one::<String>("output").unwrap()),
        wait_time: Duration::from_millis(
            matches.get_one::<String>("wait").unwrap().parse()
                .context("Invalid wait time")?
        ),
        window_title: matches.get_one::<String>("window-title").cloned(),
        teapot_executable: PathBuf::from(matches.get_one::<String>("executable").unwrap()),
        auto_close: !matches.get_flag("keep-open"),
        filename_prefix: matches.get_one::<String>("prefix").unwrap().to_string(),
        clean_before_capture: !matches.get_flag("no-clean"),
    };

    capture_teapot_screenshot(config)
}

fn capture_teapot_screenshot(config: ScreenshotConfig) -> Result<()> {
    // Create output directory
    std::fs::create_dir_all(&config.output_dir)
        .context("Failed to create output directory")?;

    // Clean existing screenshots if requested
    if config.clean_before_capture {
        println!("Cleaning existing screenshots from: {:?}", config.output_dir);
        if config.output_dir.exists() {
            for entry in std::fs::read_dir(&config.output_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "png") {
                    if let Err(e) = std::fs::remove_file(&path) {
                        println!("Warning: Failed to remove {}: {}", path.display(), e);
                    } else {
                        println!("Removed: {}", path.display());
                    }
                }
            }
        }
    }

    println!("Starting teapot application: {:?}", config.teapot_executable);

    // Launch teapot application
    let mut child = ProcessCommand::new(&config.teapot_executable)
        .spawn()
        .context("Failed to start teapot application")?;

    // Wait for the application to start and render
    println!("Waiting {} ms for application to initialize...", config.wait_time.as_millis());
    thread::sleep(config.wait_time);

    let result = capture_application_window(&config, child.id());

    // Close application if requested
    if config.auto_close {
        println!("Closing teapot application...");
        let _ = child.kill();
        let _ = child.wait();
    } else {
        println!("Application left running (use --keep-open=false to auto-close)");
    }

    result
}

fn capture_application_window(config: &ScreenshotConfig, process_id: u32) -> Result<()> {
    #[cfg(windows)]
    {
        capture_windows_window(config, process_id)
    }
    
    #[cfg(not(windows))]
    {
        capture_fullscreen_fallback(config)
    }
}

#[cfg(windows)]
fn capture_windows_window(config: &ScreenshotConfig, process_id: u32) -> Result<()> {
    use winapi::um::winuser::{EnumWindows, GetWindowThreadProcessId};
    use winapi::shared::windef::HWND;
    use std::sync::{Arc, Mutex};

    // Find window by process ID
    let target_hwnd: Arc<Mutex<Option<HWND>>> = Arc::new(Mutex::new(None));
    let target_hwnd_clone = target_hwnd.clone();
    let target_pid = process_id;

    extern "system" fn enum_window_proc(hwnd: HWND, lparam: isize) -> i32 {
        unsafe {
            let target_data = &*(lparam as *const (Arc<Mutex<Option<HWND>>>, u32));
            let (target_hwnd, target_pid) = target_data;
            
            let mut window_pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut window_pid);
            
            if window_pid == *target_pid && IsWindowVisible(hwnd) != 0 {
                *target_hwnd.lock().unwrap() = Some(hwnd);
                return 0; // Stop enumeration
            }
        }
        1 // Continue enumeration
    }

    let enum_data = (target_hwnd_clone, target_pid);
    let enum_data_ptr = &enum_data as *const _ as isize;

    unsafe {
        EnumWindows(Some(enum_window_proc), enum_data_ptr);
    }

    let hwnd = target_hwnd.lock().unwrap()
        .ok_or_else(|| anyhow::anyhow!("Could not find teapot application window"))?;

    // Get window rectangle
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return Err(anyhow::anyhow!("Failed to get window rectangle"));
        }
    }

    let width = (rect.right - rect.left) as u32;
    let height = (rect.bottom - rect.top) as u32;

    println!("Found teapot window: {}x{} at ({}, {})", width, height, rect.left, rect.top);

    // Capture the specific window area
    capture_screen_region(config, rect.left, rect.top, width, height)
}

#[cfg(not(windows))]
fn capture_fullscreen_fallback(config: &ScreenshotConfig) -> Result<()> {
    println!("Using fullscreen capture (platform-specific window capture not implemented)");
    
    let screen = screenshots::Screen::all()
        .context("Failed to get screen information")?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No screens found"))?;

    let image = screen.capture()
        .context("Failed to capture screen")?;

    save_screenshot(config, image)
}

#[cfg(windows)]
fn capture_screen_region(config: &ScreenshotConfig, x: i32, y: i32, width: u32, height: u32) -> Result<()> {
    let screen = screenshots::Screen::all()
        .context("Failed to get screen information")?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No screens found"))?;

    // Capture full screen first, then crop
    let full_image = screen.capture()
        .context("Failed to capture screen")?;

    // Convert to image crate format for cropping
    let img_buffer: RgbaImage = ImageBuffer::from_raw(
        full_image.width(),
        full_image.height(),
        full_image.rgba().to_vec()
    ).ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;

    // Crop to window area (with bounds checking)
    let crop_x = std::cmp::max(0, x as u32);
    let crop_y = std::cmp::max(0, y as u32);
    let crop_width = std::cmp::min(width, full_image.width() - crop_x);
    let crop_height = std::cmp::min(height, full_image.height() - crop_y);

    let cropped = image::imageops::crop_imm(&img_buffer, crop_x, crop_y, crop_width, crop_height);

    // Convert back to screenshots format
    let cropped_data: Vec<u8> = cropped.to_image().into_raw();
    let cropped_screenshot = screenshots::Image::new(crop_width, crop_height, cropped_data);

    save_screenshot(config, cropped_screenshot)
}

fn save_screenshot(config: &ScreenshotConfig, image: screenshots::Image) -> Result<()> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.png", config.filename_prefix, timestamp);
    let filepath = config.output_dir.join(filename);

    // Convert to PNG and save
    let img_buffer: RgbaImage = ImageBuffer::from_raw(
        image.width(),
        image.height(),
        image.rgba().to_vec()
    ).ok_or_else(|| anyhow::anyhow!("Failed to create image buffer for saving"))?;

    img_buffer.save(&filepath)
        .context("Failed to save screenshot")?;

    println!("Screenshot saved: {:?}", filepath);
    println!("Image dimensions: {}x{}", image.width(), image.height());

    // Analyze the screenshot content
    match analyze_screenshot(&filepath) {
        Ok(analysis) => {
            println!("\n{}", analysis);
        }
        Err(e) => {
            println!("Warning: Could not analyze screenshot: {}", e);
        }
    }

    Ok(())
}
