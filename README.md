# Rusteroids

A modular ECS-based Asteroids game showcasing a reusable Rust game engine architecture. The project demonstrates clean separation between generic engine components and application-specific game logic, with a focus on modern Vulkan rendering infrastructure.

## Current Status: Infrastructure Complete ✅

The project has evolved into a comprehensive rendering engine with:
- **Material System**: Modular architecture supporting StandardPBR, Unlit, and Transparent materials
- **Pipeline Management**: Multi-pipeline system with automatic shader configuration
- **UBO Architecture**: Uniform buffer objects for efficient GPU data management
- **Academic Standards**: Johannes Unterguggenberger guide-compliant projection matrices
- **Vulkan Validation**: Clean validation with best practices integration

**Working Demo**: The teapot application successfully renders 18,960 vertex models with world-space lighting, perfect aspect ratio handling, and smooth performance.

## Architecture Overview

The project is structured as a multi-crate workspace with clear separation of concerns:
- **RustEngine**: Generic, reusable game engine components with advanced rendering
- **Asteroids Game**: Application-specific implementation using the engine
- **Teapot Demo**: Testing application for rendering infrastructure validation

### Key Features Implemented

**Engine Layer:**
- 🎨 **Advanced Vulkan Rendering**: UBO-based architecture with material system and pipeline management
- 🏗️ **Academic Standards**: Projection matrices following established academic/industry guidelines
- 🎯 **Material System**: Support for PBR, unlit, and transparent material workflows
- 📐 **Perfect Aspect Ratios**: Dynamic viewport handling with coordinate system standardization
- ⚡ **Performance Optimized**: Efficient resource management with RAII patterns and minimal allocations
- 🔍 **Validation Integration**: Comprehensive Vulkan validation with synchronization best practices

**Demo Application:**
- 🫖 **3D Mesh Rendering**: OBJ loading with proper vertex/normal data handling
- 💡 **World-Space Lighting**: Fixed directional lighting that stays consistent during object rotation
- 📷 **Camera System**: Standard Y-up view space with academic projection matrices
- 🖼️ **Window Management**: Perfect resize handling with maintained aspect ratios
- 🎮 **Interactive Demo**: Real-time teapot rendering with smooth 60+ FPS performance

## Documentation

### Core Architecture Documents
- [**Project Status**](docs/PROJECT_STATUS.md) - Current achievements and development readiness
- [**Graphics Pipeline Redesign**](docs/GRAPHICS_PIPELINE_REDESIGN.md) - UBO system and material architecture
- [**Vulkan Design**](docs/VULKAN_DESIGN.md) - Core architecture and resource management patterns
- [**Vulkan Synchronization**](docs/VULKAN_SYNCHRONIZATION_ANALYSIS.md) - Best practices and validation integration

### Implementation Guides
- [**Material System**](crates/rust_engine/src/render/material/) - Complete material architecture documentation
- [**Pipeline Management**](crates/rust_engine/src/render/pipeline/) - Multi-pipeline system implementation
- [**Lighting System Updates**](docs/lighting-system-updates.md) - World-space lighting implementation
- [**Recent Updates Summary**](docs/RECENT_UPDATES_SUMMARY.md) - Comprehensive change log

### Technical Deep Dives
- [**Coordinate Space Fix**](docs/lighting-coordinate-space-fix.md) - Normal matrix calculation and GLSL compatibility
- [**Rendering Quality**](docs/lighting-system-updates.md) - Visual quality considerations and optimization strategies

## Current Project Structure

```
rusteroids/
├── src/                    # Current monolithic structure (to be refactored)
├── resources/              # Game assets
│   ├── shaders/           # GLSL shaders
│   ├── models/            # 3D models
│   ├── textures/          # Texture assets
│   └── audio/             # Sound files
├── docs/                  # Architecture documentation
│   ├── diagrams/          # SVG diagrams
## Current Project Structure

```
rusteroids/
├── crates/                  # Multi-crate workspace
│   ├── rust_engine/        # ✅ Core engine with advanced rendering
│   └── asteroids/          # 🔄 Game implementation (planned)
├── teapot_app/             # ✅ Working rendering demo
├── resources/              # ✅ Game assets and compiled shaders
│   ├── shaders/           # ✅ GLSL shaders (vert_ubo.vert, frag_ubo_simple.spv)
│   ├── models/            # ✅ 3D models (teapot_with_normals.obj)
│   ├── textures/          # 📁 Texture assets (prepared)
│   └── audio/             # 📁 Sound files (prepared)
├── tools/                  # 🔧 Development utilities
├── docs/                   # ✅ Comprehensive architecture documentation
└── target/                 # ✅ Build output with compiled shaders
```

## Getting Started

### Prerequisites
- Rust (latest stable version)
- Vulkan SDK installed and configured
- GLFW-compatible system (Windows, Linux, macOS)

### Quick Start
```bash
# Clone the repository
git clone https://github.com/yourusername/rusteroids.git
cd rusteroids

# Run the working teapot demo
cargo run --bin teapot_app

# Build the entire workspace
cargo build
```

### Development Setup
```bash
# Enable Vulkan validation layers for development
set VK_LAYER_PATH=path/to/vulkan/sdk/bin

# Run with validation (debug builds have validation enabled)
cargo run --bin teapot_app

# Check for compilation errors across all crates
cargo check --workspace
```

## Infrastructure Highlights

### Material System Architecture
```rust
// Supported material types
enum MaterialType {
    StandardPBR,     // Full physically-based rendering
    Unlit,          // Simple unshaded materials (currently active)
    TransparentPBR, // PBR with alpha blending
    TransparentUnlit, // Unshaded with transparency
}
```

### Pipeline Management
```rust
// Multi-pipeline support for different material types
pub struct PipelineManager {
    pipelines: HashMap<MaterialType, GraphicsPipeline>,
    active_pipeline: Option<MaterialType>,
}
```

### UBO Architecture
```glsl
// Set 0: Per-frame data (camera, lighting)
layout(set = 0, binding = 0) uniform CameraUBO { ... };
layout(set = 0, binding = 1) uniform LightingUBO { ... };

// Set 1: Per-material data (prepared for future features)
layout(set = 1, binding = 0) uniform MaterialUBO { ... };
```

## Performance & Quality

- **Rendering**: 60+ FPS with 18,960 vertex teapot model
- **Memory**: Efficient Vulkan resource management with RAII patterns
- **Validation**: Clean Vulkan validation with best practices integration
- **Aspect Ratios**: Perfect handling during window resize operations
- **Lighting**: World-space lighting with proper normal transformations

## Development Status

### ✅ Completed Infrastructure
- Material system with modular architecture
- Multi-pipeline management system
- UBO-based rendering with descriptor sets
- Academic-standard projection matrices
- World-space lighting implementation
- Vulkan validation integration

### 🔄 Next Development Phase
- Material UBO integration (Set 1 descriptors)
- Texture system implementation
- Multi-light support expansion
- PBR material workflow completion

## Contributing

This project follows Rust best practices and emphasizes:
- **Safety**: Memory-safe resource management with RAII patterns
- **Performance**: Efficient Vulkan usage with minimal allocations
- **Maintainability**: Modular architecture with clear separation of concerns
- **Standards**: Academic compliance and industry best practices

See the [documentation](docs/) for detailed architecture guides and implementation details.

## License

MIT License - see LICENSE file for details.
