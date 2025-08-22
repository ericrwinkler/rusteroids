# Rusteroids Documentation Index

## 📖 Quick Navigation

This index provides easy access to all project documentation, organized by purpose and audience.

## 🎯 Project Overview

### Essential Reading
- **[README.md](../README.md)** - Project overview, current status, and getting started
- **[Project Status](PROJECT_STATUS.md)** - Comprehensive current achievements and development readiness
- **[Development Roadmap](DEVELOPMENT_ROADMAP.md)** - Next phases, timelines, and priorities

### Recent Changes
- **[Recent Updates Summary](RECENT_UPDATES_SUMMARY.md)** - Latest achievements and infrastructure transformation
- **[Graphics Pipeline Redesign](GRAPHICS_PIPELINE_REDESIGN.md)** - UBO implementation and material system

## 🏗️ Architecture Documentation

### Core Systems
- **[Vulkan Design](VULKAN_DESIGN.md)** - Core architecture patterns and resource management
- **[Vulkan Synchronization Analysis](VULKAN_SYNCHRONIZATION_ANALYSIS.md)** - Best practices and validation integration

### Rendering Systems
- **[Lighting System Updates](lighting-system-updates.md)** - World-space lighting implementation and future work
- **[Lighting Coordinate Space Fix](lighting-coordinate-space-fix.md)** - Normal matrix calculation and GLSL compatibility

## 🔧 Implementation Guides

### Material System
Located in: `crates/rust_engine/src/render/material/`
- **[material_type.rs](../crates/rust_engine/src/render/material/material_type.rs)** - MaterialType enum and conversions
- **[material_ubo.rs](../crates/rust_engine/src/render/material/material_ubo.rs)** - GPU uniform buffer structures  
- **[material_manager.rs](../crates/rust_engine/src/render/material/material_manager.rs)** - Resource management system
- **[texture_manager.rs](../crates/rust_engine/src/render/material/texture_manager.rs)** - Texture system placeholder

### Pipeline Management
Located in: `crates/rust_engine/src/render/pipeline/`
- **[pipeline_manager.rs](../crates/rust_engine/src/render/pipeline/pipeline_manager.rs)** - Multi-pipeline management
- **[pipeline_config.rs](../crates/rust_engine/src/render/pipeline/pipeline_config.rs)** - Configuration and shader paths

### Core Rendering
Located in: `crates/rust_engine/src/render/`
- **[mod.rs](../crates/rust_engine/src/render/mod.rs)** - High-level rendering interface
- **[vulkan/renderer.rs](../crates/rust_engine/src/render/vulkan/renderer.rs)** - VulkanRenderer implementation

## 🎮 Applications

### Demo Applications
- **[Teapot Demo](../teapot_app/src/main.rs)** - Working rendering demonstration with 3D mesh and lighting
- **[Teapot Application Guide](APPLICATION_GUIDE.md)** - How to run and understand the demo *(planned)*

### Game Implementation
- **[Asteroids Game](../crates/asteroids/)** - Game-specific implementation *(future)*
- **[ECS Architecture](ECS_DESIGN.md)** - Entity-Component-System design *(planned)*

## 🛠️ Development Resources

### Setup and Building
- **[Development Setup](DEVELOPMENT_SETUP.md)** - Environment configuration and tools *(planned)*
- **[Build Guide](BUILD_GUIDE.md)** - Compilation and dependency management *(planned)*
- **[Testing Guide](TESTING_GUIDE.md)** - Testing strategy and validation *(planned)*

### Performance and Optimization
- **[Performance Guide](PERFORMANCE_GUIDE.md)** - Optimization strategies and benchmarks *(planned)*
- **[Memory Management](MEMORY_MANAGEMENT.md)** - Resource allocation and cleanup patterns *(planned)*

## 📚 Technical References

### Vulkan and Graphics
- **[Shader System](SHADER_SYSTEM.md)** - GLSL compilation and management *(planned)*
- **[Coordinate Systems](COORDINATE_SYSTEMS.md)** - Academic standards and transformations *(documented in pipeline redesign)*
- **[UBO Architecture](UBO_ARCHITECTURE.md)** - Uniform buffer design patterns *(documented in pipeline redesign)*

### Academic References
- **[Johannes Unterguggenberger's Projection Matrix Guide](https://johannesugb.github.io/gpu-programming/setting-up-a-proper-vulkan-projection-matrix/)** - Referenced standard for projection matrices
- **[Vulkan Synchronization Validation](https://johannesugb.github.io)** - Synchronization best practices reference

## 🗂️ Documentation Status

### ✅ Complete and Current
- Project overview and status documentation
- Core architecture design documents
- Material system and pipeline management guides
- Recent updates and change summaries
- Development roadmap with clear phases

### 🔄 In Progress
- Performance optimization guides
- Advanced feature documentation updates
- Tutorial and getting started content

### 📋 Planned
- Complete developer onboarding guides
- Performance benchmarking documentation
- Advanced rendering technique guides
- Game implementation architecture

## 🎯 Documentation by Audience

### For New Developers
1. **[README.md](../README.md)** - Start here for project overview
2. **[Project Status](PROJECT_STATUS.md)** - Understand current capabilities
3. **[Development Roadmap](DEVELOPMENT_ROADMAP.md)** - See where the project is heading
4. **[Vulkan Design](VULKAN_DESIGN.md)** - Core architecture patterns

### For Graphics Programmers
1. **[Graphics Pipeline Redesign](GRAPHICS_PIPELINE_REDESIGN.md)** - UBO architecture and material system
2. **[Lighting System Updates](lighting-system-updates.md)** - Lighting implementation details
3. **[Vulkan Synchronization Analysis](VULKAN_SYNCHRONIZATION_ANALYSIS.md)** - Best practices integration
4. **Material/Pipeline source code** - Implementation reference

### For Game Developers
1. **[README.md](../README.md)** - Feature overview and current capabilities
2. **[Teapot Demo](../teapot_app/src/main.rs)** - Working example of engine usage
3. **[ECS Design](ECS_DESIGN.md)** - Game object architecture *(planned)*
4. **[Asset Pipeline](ASSET_PIPELINE.md)** - Content creation workflow *(planned)*

### For Project Maintainers
1. **[Recent Updates Summary](RECENT_UPDATES_SUMMARY.md)** - Track all changes and achievements
2. **[Development Roadmap](DEVELOPMENT_ROADMAP.md)** - Plan future development phases
3. **[Architecture Documents](VULKAN_DESIGN.md)** - Maintain design consistency
4. **[Performance Considerations](PERFORMANCE_GUIDE.md)** - Optimization strategy *(planned)*

## 🔗 External Resources

### Vulkan Learning
- [Vulkan Tutorial](https://vulkan-tutorial.com/) - Comprehensive Vulkan learning resource
- [Vulkan Specification](https://www.khronos.org/vulkan/) - Official API reference
- [LunarG Validation Layers](https://vulkan.lunarg.com/doc/sdk) - Debugging and validation tools

### Rust Graphics
- [ash](https://docs.rs/ash/) - Vulkan bindings for Rust
- [nalgebra](https://nalgebra.org/) - Linear algebra library
- [bytemuck](https://docs.rs/bytemuck/) - Safe casting for GPU data

### Graphics Programming
- [Real-Time Rendering](http://www.realtimerendering.com/) - Graphics programming reference
- [Physically Based Rendering](https://pbr-book.org/) - Advanced rendering techniques
- [GPU Gems](https://developer.nvidia.com/gpugems) - NVIDIA graphics programming resources

---

## 📝 Documentation Maintenance

This index is updated whenever new documentation is added or existing documentation is reorganized. Last updated: August 2025

For questions about documentation organization or suggestions for new documentation, see the project's contribution guidelines or open an issue.
