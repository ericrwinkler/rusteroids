# Screenshot Tool

An automated screenshot capture tool for the Rusteroids teapot rendering application. This tool helps validate that the Vulkan rendering engine is working correctly by capturing and analyzing screenshots.

## Quick Start

```cmd
:: Use the validation script for easy screenshot capture
.\validate_rendering.bat baseline     :: Before changes
.\validate_rendering.bat validation   :: After changes
.\validate_rendering.bat material     :: Material system changes

:: Analyze any screenshot
cargo run -- --analyze "screenshots\screenshot_name.png"
```

## Features

- **Automated Screenshot Capture**: Launches the teapot app and captures screenshots automatically
- **Window Detection**: Finds and captures specific application windows (Windows-specific)
- **Content Analysis**: Analyzes screenshots to determine if they show rendered 3D content
- **Auto-Cleanup**: Removes old screenshots before capturing new ones
- **Validation Script**: Integrated batch script for AI development workflow
- **Configurable Timing**: Adjustable wait times for application startup
- **Multiple Output Formats**: Supports PNG output with timestamped filenames

## Usage

### Basic Usage
```bash
# Capture a screenshot with default settings
./screenshot_tool.exe

# Capture with custom wait time (5 seconds)
./screenshot_tool.exe --wait 5000

# Keep the application running after screenshot
./screenshot_tool.exe --keep-open

# Custom output directory and filename prefix
./screenshot_tool.exe --output "./test_screenshots" --prefix "render_test"
```

### Command Line Options

- `-o, --output <DIR>`: Output directory for screenshots (default: "screenshots")
- `-w, --wait <MILLISECONDS>`: Time to wait before taking screenshot (default: 3000)
- `-t, --window-title <TITLE>`: Window title to search for (optional)
- `-e, --executable <PATH>`: Path to teapot executable (default: "target/debug/teapot_app.exe")
- `--keep-open`: Keep the application running after screenshot
- `-p, --prefix <PREFIX>`: Filename prefix for screenshots (default: "teapot")

## Screenshot Analysis

The tool automatically analyzes captured screenshots and provides:

- **Dimensions and pixel counts**
- **Color distribution analysis** (black, white, colored pixels)
- **Average brightness calculation**
- **Content classification**:
  - ✅ **RenderedScene**: Normal 3D rendering detected
  - ⚠️ **BlankOrEmpty**: Screenshot appears blank or empty
  - ❌ **LoadingOrError**: May show loading screen or error state
  - ❓ **UnknownPattern**: Unexpected content pattern

## Example Output

```
Starting teapot application: "target/debug/teapot_app.exe"
Waiting 3000 ms for application to initialize...
[Application startup logs...]
Found teapot window: 800x600 at (100, 100)
Screenshot saved: "screenshots/teapot_20250904_204303.png"
Image dimensions: 800x600

Screenshot Analysis:
  Dimensions: 800x600
  Total pixels: 480000
  Black pixels: 15.2%
  White pixels: 8.7%
  Colored pixels: 68.3%
  Average brightness: 127/255
  Content classification: RenderedScene
  ✅ Screenshot appears to show rendered 3D content
Closing teapot application...
```

## Requirements

- Windows (for window-specific capture functionality)
- Rust with Cargo
- Vulkan-capable graphics hardware
- The Rusteroids teapot application built and available

## Building

```bash
cargo build --release
```

## Use Cases

1. **Automated Testing**: Verify rendering output in CI/CD pipelines
2. **Regression Testing**: Compare screenshots across code changes
3. **Development Validation**: Quick visual verification during development
4. **Performance Testing**: Capture output during performance benchmarks
5. **Bug Reporting**: Automatically capture evidence of rendering issues

## Technical Details

- Uses the `screenshots` crate for cross-platform screen capture
- Windows-specific window detection using WinAPI
- Image analysis using the `image` crate
- Supports RGBA image processing and PNG output
- Automatic process management with configurable cleanup

## Troubleshooting

- **No window found**: Ensure the teapot application is building and running correctly
- **Black screenshots**: Check Vulkan driver installation and application startup time
- **Permission errors**: Run from the project root directory with proper file permissions
- **Build errors**: Ensure all dependencies are available and up to date
