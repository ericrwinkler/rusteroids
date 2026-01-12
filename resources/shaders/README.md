# Shader Organization

This directory contains modular GLSL shaders organized by function and feature.

## Structure

```
shaders/
├── common/              # Shared definitions and utilities
│   ├── camera.glsl      # Camera UBO (Set 0, Binding 0)
│   ├── lighting.glsl    # Light structures and UBO (Set 0, Binding 1)
│   ├── material.glsl    # Material UBO and textures (Set 1)
│   └── utils.glsl       # Common utility functions
│
├── vertex/              # Vertex shader modules
│   ├── instancing.glsl  # Instance attribute definitions and helpers
│   └── standard.vert    # Standard vertex shader with instancing
│
└── fragment/            # Fragment shader modules
    ├── pbr/             # Physically-based rendering
    │   ├── brdf.glsl    # BRDF functions (Fresnel, GGX, etc.)
    │   ├── pbr_core.glsl # Core PBR lighting calculations
    │   └── pbr_main.frag # Main PBR fragment shader
    │
    └── unlit/           # Unlit rendering
        └── unlit_main.frag # Simple unlit fragment shader
```

## Active Shaders

These are the shaders currently compiled and used by the engine:

### PBR Pipeline (StandardPBR, TransparentPBR)
- **Vertex**: `vertex/standard.vert`
- **Fragment**: `fragment/pbr/pbr_main.frag`
- **Compiled to**: `target/shaders/standard_pbr_vert.spv`, `standard_pbr_frag.spv`

### Unlit Pipeline (Unlit, TransparentUnlit, Skybox)
- **Vertex**: `vertex/standard.vert` (reused)
- **Fragment**: `fragment/unlit/unlit_main.frag`
- **Compiled to**: `target/shaders/standard_pbr_vert.spv`, `unlit_frag.spv`

## Design Principles

### Separation of Concerns
- **Common**: Shared data structures used across multiple shaders
- **Vertex**: Vertex transformation and attribute processing
- **Fragment**: Pixel/fragment shading separated by technique

### Modularity
- Small, focused include files for specific features
- Reusable functions and definitions
- Easy to maintain and extend

### Feature-Based Organization
- PBR features grouped under `fragment/pbr/`
- Lighting calculations separate from BRDF math
- Unlit rendering isolated for simplicity

## Usage

Shaders use `#include` directives (processed by shader compiler) to compose functionality:

```glsl
#version 450
#include "../common/camera.glsl"
#include "../common/material.glsl"
#include "./pbr_core.glsl"
```

## Compilation

Shaders are compiled via `build.rs` using `glslc` (from Vulkan SDK):
- Source: `resources/shaders/**/*.{vert,frag}`
- Output: `target/shaders/**/*.spv`

The build system automatically handles `#include` directives during compilation.

## Legacy Files Removed

The following monolithic shaders have been deleted (no longer used):
- `frag_material_ubo.frag`
- `frag_ubo_simple.frag`
- `multi_light_frag.frag`
- `multi_light_vert.vert`
- `ui_simple_frag.frag`
- `ui_simple_vert.vert`
- `ui_text_frag.frag`
- `ui_text_vert.vert`
- `vert_material_ubo.vert`
- `vert_ubo.vert`
