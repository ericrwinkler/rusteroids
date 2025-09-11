# Phase 0: Clean Up and Baseline Validation - Action Plan

## Step 0.1: Fix Current Build Issues ⚠️ 

**Goal**: Get the teapot app building and running cleanly so we have a stable baseline.

### Current Status Check

First, let's verify what's broken:

```cmd
# Check current build status
cd c:\Users\Eric_\Projects\rusteroids
cargo build
```

### Fix 1: Correct Shader Paths in Teapot App

The shader path issue is already fixed, but let's verify:

**File**: `teapot_app/src/main.rs` (around line 47)
```rust
// Should be:
.with_shader_paths(
    "../target/shaders/vert_ubo.spv".to_string(),
    "../target/shaders/frag_ubo_simple.spv".to_string()
)
```

### Fix 2: Remove Broken Multi-Light Shader (Temporary)

The `multi_light_frag.frag` shader is causing build failures. Let's temporarily remove it:

**File**: `resources/shaders/multi_light_frag.frag` - DELETE or RENAME to `.bak`
**File**: `resources/shaders/multi_light_vert.vert` - DELETE or RENAME to `.bak`

### Fix 3: Comment Out Broken Demo Apps

**Files to comment out in Cargo.toml**:
- `teapot_app/Cargo.toml` - Remove or comment broken demo binaries
- Or individual demo files - Add `#[cfg(feature = "broken")]` to disable

### **Verification Step 1**: Clean Build

```cmd
# From project root
cargo build
# Expected: Clean build with no errors
```

### **Verification Step 2**: Teapot App Runs Successfully  

```cmd
cd teapot_app
cargo run
# Expected: Window opens, teapot renders with rotating materials, 60+ FPS
```

## Step 0.2: Establish Reference Screenshots and Metrics 📸

### **Verification Step 3**: Capture Baseline Screenshots

```cmd
# From project root, with teapot app running
cd tools\screenshot_tool
cargo run -- --prefix "baseline_phase0" --wait 5000
```

**Expected Output**:
```
Content Classification: RenderedScene
Colored Pixels: >60%
Average Brightness: 50-200
Screenshot saved: baseline_phase0_[timestamp].png
```

### **Verification Step 4**: Document Current Light Parameters

**Create**: `docs/CURRENT_BASELINE_LIGHT_PARAMETERS.md`

```rust
// EXACT current light parameters (from teapot_app/src/main.rs around line 115)
pub const BASELINE_LIGHT_DIRECTION: Vec3 = Vec3::new(-0.7, -1.0, 0.3);
pub const BASELINE_LIGHT_COLOR: Vec3 = Vec3::new(1.0, 0.95, 0.9);
pub const BASELINE_LIGHT_INTENSITY: f32 = 1.5;

// EXACT ambient parameters
pub const BASELINE_AMBIENT_COLOR: Vec3 = Vec3::new(0.15, 0.12, 0.18);
pub const BASELINE_AMBIENT_INTENSITY: f32 = 0.1;
```

### **Verification Step 5**: Performance Baseline

Add simple FPS counter to teapot app for baseline measurement:

**File**: `teapot_app/src/main.rs` - Add to main loop:

```rust
// Add to TeapotApp struct
pub struct TeapotApp {
    // ... existing fields ...
    frame_count: u64,
    last_fps_time: Instant,
    fps: f32,
}

// Add to update method
pub fn update(&mut self, delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
    self.frame_count += 1;
    
    // Calculate FPS every second
    if self.last_fps_time.elapsed().as_secs_f32() >= 1.0 {
        self.fps = self.frame_count as f32 / self.last_fps_time.elapsed().as_secs_f32();
        log::info!("FPS: {:.1}", self.fps);
        self.frame_count = 0;
        self.last_fps_time = Instant::now();
    }
    
    // ... rest of existing update code ...
}
```

### **Verification Step 6**: All Materials Cycle Correctly

Let the teapot app run for 15+ seconds and verify all 5 materials show:
- Material 1: Ruby (red)
- Material 2: Chrome (metallic gray) 
- Material 3: Copper (orange/brown)
- Material 4: Gold (yellow)
- Material 5: Jade (green)

## Success Criteria for Phase 0 ✅

### **Build Success**:
```cmd
cargo build
# Output: "Finished `dev` profile [unoptimized + debuginfo] target(s) in X.XXs"
# No errors, warnings OK
```

### **Runtime Success**:
```cmd
cd teapot_app
cargo run
# Output: 
# - Window opens immediately
# - Teapot renders with correct lighting
# - Materials cycle every 3 seconds
# - FPS consistently >60
# - No Vulkan validation errors (warnings OK)
```

### **Screenshot Validation**:
```cmd
cd tools\screenshot_tool  
cargo run -- --analyze "../validation/baseline_phase0_[timestamp].png"
# Output:
# Content Classification: RenderedScene ✅
# Colored Pixels: >60% ✅ 
# Average Brightness: 50-200 ✅
```

### **Performance Baseline**:
- **Target FPS**: 60+ sustained
- **Frame Time**: <16ms average
- **Material Switch**: Smooth transitions every 3 seconds

## Immediate Next Step Commands

Run these commands in sequence to complete Phase 0:

```powershell
# 1. Check current status
cd C:\Users\Eric_\Projects\rusteroids
cargo build

# 2. If build fails, fix shader paths in teapot_app/src/main.rs (already done)

# 3. Remove broken multi-light shaders temporarily
cd resources\shaders
if exist multi_light_frag.frag (ren multi_light_frag.frag multi_light_frag.frag.bak)
if exist multi_light_vert.vert (ren multi_light_vert.vert multi_light_vert.vert.bak)

# 4. Build again
cd ..\..
cargo build

# 5. Run teapot app to verify baseline
cd teapot_app
cargo run

# 6. Capture baseline screenshot (in separate terminal while app runs)
cd ..\tools\screenshot_tool
cargo run -- --prefix "baseline_phase0" --wait 5000
```

## What This Achieves

After completing Phase 0, you'll have:

1. **Stable Build**: All code compiles without errors
2. **Working Teapot App**: Renders identically to before cleanup
3. **Performance Baseline**: Known FPS and frame time metrics  
4. **Visual Baseline**: Reference screenshots for all materials
5. **Documented Parameters**: Exact light values to preserve in entity system

This gives us a **rock-solid foundation** to build the lights-as-entities system on, with comprehensive validation to catch any regressions immediately.

**Next**: Once Phase 0 is complete, we move to Phase 1.1 (Multi-Light Data Structures) which can be implemented without changing any rendering behavior.
