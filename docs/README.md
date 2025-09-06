# Rusteroids Documentation Index

## 📖 Quick Navigation

This documentation provides comprehensive coverage of the Rusteroids Vulkan rendering engine architecture, development status, and implementation guides.

## 🎯 Essential Reading

### Primary Documentation
- **[README.md](../README.md)** - Project overview, current status, and getting started
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Complete technical architecture and implementation details
- **[DEVELOPMENT.md](DEVELOPMENT.md)** - Development status, roadmap, and contribution guidelines

### Specialized Documentation
- **[LIGHTING_AND_SHADER_REFACTOR_PLAN.md](LIGHTING_AND_SHADER_REFACTOR_PLAN.md)** - Comprehensive lighting system refactor plan
- **[VULKAN_SYNCHRONIZATION_ANALYSIS.md](VULKAN_SYNCHRONIZATION_ANALYSIS.md)** - Vulkan synchronization patterns and best practices
- **[SCREENSHOT_VALIDATION_WORKFLOW.md](SCREENSHOT_VALIDATION_WORKFLOW.md)** - Development workflow and quality assurance

## 🏗️ Architecture Documentation

### Core Systems (See ARCHITECTURE.md)
- **Material System**: Complete modular architecture with MaterialType enum, UBO structures, resource management
- **Pipeline Management**: Multi-pipeline system supporting StandardPBR, Unlit, TransparentPBR, TransparentUnlit
- **UBO Architecture**: Set 0 (camera/lighting) and Set 1 (materials) descriptor layouts
- **Resource Management**: RAII patterns with proper Drop implementations

### Rendering Pipeline (See ARCHITECTURE.md)
- **Coordinate Systems**: Academic-standard projection matrices with Vulkan compatibility
- **Lighting System**: World-space lighting with proper normal matrix transformations
- **Aspect Ratio Handling**: Dynamic viewport system for perfect window resizing

## � Development Status & Roadmap (See DEVELOPMENT.md)

### Current Status: Infrastructure Complete ✅
- **Phase 5**: Material UBO system with multiple materials demo
- **Performance**: 60+ FPS with clean Vulkan validation
- **Quality**: Academic-standard projection matrices and world-space lighting

### Next Priority: Lighting System Refactor
- **Timeline**: 6 weeks comprehensive overhaul
- **Goals**: Multi-light support, shadow mapping, shader organization
- **Success Criteria**: 4-8 lights, teapot self-shadowing, 60+ FPS

## 🎮 Applications

### Demo Applications
- **[Teapot Demo](../teapot_app/src/main.rs)** - Working multi-material rendering demonstration
- Current: 5 materials with automatic switching, PBR shading, clean validation

### Game Implementation (Future)
- **Asteroids Game**: Classic gameplay with advanced rendering
- **ECS Foundation**: Parallel development with rendering system

## 🛠️ Development Workflow

### Quality Assurance
- **[Screenshot Validation Workflow](SCREENSHOT_VALIDATION_WORKFLOW.md)** - Mandatory validation for all rendering changes
- **Pre-commit validation**: Capture baseline and validation screenshots
- **Automated analysis**: RenderedScene detection with quality metrics

### Development Environment
- **Build System**: Cargo with multi-crate workspace
- **Testing**: Teapot demo for rendering validation
- **Tools**: Screenshot validation, shader compilation, asset processing

## 📚 Technical References & Historical Notes

### Specialized Analysis
- **[Vulkan Synchronization Analysis](VULKAN_SYNCHRONIZATION_ANALYSIS.md)** - Pipeline barriers and memory coherency
- **[Lighting Refactor Plan](LIGHTING_AND_SHADER_REFACTOR_PLAN.md)** - Comprehensive multi-light system design

### Historical Technical Notes
Located in: `technical-notes/`
- **[lighting-coordinate-space-fix.md](technical-notes/lighting-coordinate-space-fix.md)** - Normal matrix debugging record
- **[lighting-system-updates.md](technical-notes/lighting-system-updates.md)** - Historical lighting planning

### External References
- **[Johannes Unterguggenberger's Projection Matrix Guide](https://johannesugb.github.io/gpu-programming/setting-up-a-proper-vulkan-projection-matrix/)** - Academic standard for projection matrices
- **[Hans-Kristian Arntzen's Vulkan Synchronization](https://themaister.net/blog/)** - Synchronization best practices

## 🎯 Documentation by Audience

### For New Developers
1. **[README.md](../README.md)** - Start here for project overview
2. **[ARCHITECTURE.md](ARCHITECTURE.md)** - Understand the technical foundation
3. **[DEVELOPMENT.md](DEVELOPMENT.md)** - See current status and next steps

### For Contributors
1. **[DEVELOPMENT.md](DEVELOPMENT.md)** - Development status and workflow
2. **[Screenshot Validation](SCREENSHOT_VALIDATION_WORKFLOW.md)** - Required validation process
3. **[Lighting Refactor Plan](LIGHTING_AND_SHADER_REFACTOR_PLAN.md)** - Next major development phase

### For Technical Review
1. **[ARCHITECTURE.md](ARCHITECTURE.md)** - Complete system design and implementation
2. **[Vulkan Synchronization Analysis](VULKAN_SYNCHRONIZATION_ANALYSIS.md)** - Low-level Vulkan patterns
3. **Technical Notes**: Historical debugging and implementation records
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
