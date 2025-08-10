<!-- Use this file to provide workspace-specific custom instructions to Copilot. For more details, visit https://code.visualstudio.com/docs/copilot/copilot-customization#_use-a-githubcopilotinstructionsmd-file -->

This workspace is for a modular ECS-based Asteroids game in Rust using Vulkan and GLFW. Use idiomatic Rust, modular design, and separation of concerns for ECS, rendering (2D/3D), input, audio, menus, config, asset loading, and save/load. Support both 2D and 3D assets, persistent settings/scores, and a resources folder for all static assets (models, textures, shaders, audio, fonts, data).

Focus on separation of concerns, modularity, saety, and performance. Use Rust's type system effectively to ensure safety and correctness. Follow best practices for ECS architecture, ensuring components are reusable and systems are decoupled.

Vulkan will be used for rndering, so ensure that the code is compatible with Vulkan's requirements. GLFW will be used for window management and input handling.

Vulkan rendering features should include:
- Support for 3D rendering.
- Support for 2D rendering.
- Efficient resource management for textures, models, and shaders.
- Support for basic lighting and shading.


When writing code, be sure to run a build and test it to ensure correctness. Use Rust's testing framework to write unit tests for components and systems.

Always read the design documents and architecture diagrams provided in the `docs/` folder to understand the overall structure and design principles of the project. Follow the coding conventions and patterns established in the existing codebase.

Do not import new Cargo.toml dependencies without asking. We want to build our own libraries for asset loading, serialization, and other features to maintain control over the codebase and ensure compatibility with the ECS architecture.