# Rusteroids Project Restructure Proposal

## Current Structure Issues

The current structure mixes engine-level code with application-specific code, making it difficult to:
- Reuse engine components in other projects
- Test engine and application independently
- Maintain clear separation of concerns
- Scale the codebase as it grows

![Module Dependencies](diagrams/module_dependencies.svg)

## Proposed New Structure

```
rusteroids/
├── Cargo.toml                     # Workspace configuration
├── Cargo.lock
├── README.md
├── LICENSE
├── build.rs                       # Workspace-level build script
├── resources/                     # Shared resources directory
│   ├── engine/                    # Engine-level assets
│   │   ├── shaders/
│   │   │   ├── common/
│   │   │   ├── debug/
│   │   │   └── util/
│   │   ├── textures/
│   │   │   ├── default/
│   │   │   └── debug/
│   │   └── fonts/
│   └── asteroids/                 # Game-specific assets
│       ├── models/
│       ├── textures/
│       ├── audio/
│       │   ├── music/
│       │   └── sfx/
│       ├── data/
│       └── ui/
├── docs/                          # Documentation
│   ├── ENGINE_DESIGN.md
│   ├── DESIGN.md                  # Rendering design doc
│   ├── API.md
│   ├── CONTRIBUTING.md
│   └── diagrams/
│       ├── engine_architecture.drawio
│       ├── resource_ownership.drawio
│       ├── data_flow.drawio
│       ├── module_dependencies.drawio
│       ├── command_recording.drawio
│       └── memory_management.drawio
├── crates/                        # Multi-crate workspace
│   ├── rust_engine/               # Generic engine crate
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── foundation/        # Core utilities
│   │   │   │   ├── mod.rs
│   │   │   │   ├── math/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── vector.rs
│   │   │   │   │   ├── matrix.rs
│   │   │   │   │   ├── quaternion.rs
│   │   │   │   │   └── transform.rs
│   │   │   │   ├── memory/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── allocator.rs
│   │   │   │   │   ├── pool.rs
│   │   │   │   │   └── arena.rs
│   │   │   │   ├── collections/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── handle_map.rs
│   │   │   │   │   ├── slot_map.rs
│   │   │   │   │   └── free_list.rs
│   │   │   │   ├── threading/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── job_system.rs
│   │   │   │   │   └── task_pool.rs
│   │   │   │   ├── time/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── timer.rs
│   │   │   │   │   └── stopwatch.rs
│   │   │   │   └── logging/
│   │   │   │       ├── mod.rs
│   │   │   │       └── structured.rs
│   │   │   ├── ecs/               # Entity-Component-System
│   │   │   │   ├── mod.rs
│   │   │   │   ├── world.rs
│   │   │   │   ├── entity.rs
│   │   │   │   ├── component.rs
│   │   │   │   ├── system.rs
│   │   │   │   ├── query.rs
│   │   │   │   ├── resource.rs
│   │   │   │   └── event.rs
│   │   │   ├── assets/            # Asset management
│   │   │   │   ├── mod.rs
│   │   │   │   ├── handle.rs
│   │   │   │   ├── loader.rs
│   │   │   │   ├── registry.rs
│   │   │   │   ├── cache.rs
│   │   │   │   ├── streaming.rs
│   │   │   │   └── hot_reload.rs
│   │   │   ├── render/            # Rendering framework
│   │   │   │   ├── mod.rs
│   │   │   │   ├── context.rs
│   │   │   │   ├── renderer.rs
│   │   │   │   ├── camera.rs
│   │   │   │   ├── mesh.rs
│   │   │   │   ├── material.rs
│   │   │   │   ├── lighting.rs
│   │   │   │   ├── scene/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── graph.rs
│   │   │   │   │   ├── node.rs
│   │   │   │   │   └── culling.rs
│   │   │   │   └── vulkan/        # Vulkan implementation
│   │   │   │       ├── mod.rs
│   │   │   │       ├── context.rs
│   │   │   │       ├── resources/
│   │   │   │       │   ├── mod.rs
│   │   │   │       │   ├── buffer.rs
│   │   │   │       │   ├── image.rs
│   │   │   │       │   ├── pipeline.rs
│   │   │   │       │   └── descriptor.rs
│   │   │   │       ├── commands/
│   │   │   │       │   ├── mod.rs
│   │   │   │       │   ├── recording.rs
│   │   │   │       │   └── submission.rs
│   │   │   │       └── memory/
│   │   │   │           ├── mod.rs
│   │   │   │           ├── allocator.rs
│   │   │   │           └── manager.rs
│   │   │   ├── input/             # Input handling
│   │   │   │   ├── mod.rs
│   │   │   │   ├── manager.rs
│   │   │   │   ├── keyboard.rs
│   │   │   │   ├── mouse.rs
│   │   │   │   ├── gamepad.rs
│   │   │   │   ├── action_map.rs
│   │   │   │   └── gesture.rs
│   │   │   ├── audio/             # Audio system
│   │   │   │   ├── mod.rs
│   │   │   │   ├── context.rs
│   │   │   │   ├── source.rs
│   │   │   │   ├── listener.rs
│   │   │   │   ├── mixer.rs
│   │   │   │   └── streaming.rs
│   │   │   ├── physics/           # Physics integration
│   │   │   │   ├── mod.rs
│   │   │   │   ├── world.rs
│   │   │   │   ├── body.rs
│   │   │   │   ├── collision.rs
│   │   │   │   └── constraint.rs
│   │   │   ├── config/            # Configuration system
│   │   │   │   ├── mod.rs
│   │   │   │   ├── loader.rs
│   │   │   │   ├── validator.rs
│   │   │   │   └── traits.rs
│   │   │   ├── platform/          # Platform abstraction
│   │   │   │   ├── mod.rs
│   │   │   │   ├── window.rs
│   │   │   │   ├── filesystem.rs
│   │   │   │   └── threading.rs
│   │   │   ├── plugin/            # Plugin system
│   │   │   │   ├── mod.rs
│   │   │   │   ├── manager.rs
│   │   │   │   ├── registry.rs
│   │   │   │   └── loader.rs
│   │   │   ├── application.rs     # Application trait and runner
│   │   │   └── engine.rs          # Main engine struct
│   │   ├── examples/              # Engine usage examples
│   │   │   ├── minimal.rs
│   │   │   ├── triangle.rs
│   │   │   └── cube.rs
│   │   └── tests/                 # Engine tests
│   │       ├── integration/
│   │       └── unit/
│   ├── asteroids/                 # Game application crate
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── lib.rs
│   │   │   ├── app.rs             # AsteroidsApp implementation
│   │   │   ├── config.rs          # Game configuration
│   │   │   ├── states/            # Game state management
│   │   │   │   ├── mod.rs
│   │   │   │   ├── main_menu.rs
│   │   │   │   ├── playing.rs
│   │   │   │   ├── paused.rs
│   │   │   │   ├── game_over.rs
│   │   │   │   └── high_scores.rs
│   │   │   ├── components/        # Game-specific components
│   │   │   │   ├── mod.rs
│   │   │   │   ├── player.rs
│   │   │   │   ├── asteroid.rs
│   │   │   │   ├── bullet.rs
│   │   │   │   ├── velocity.rs
│   │   │   │   ├── health.rs
│   │   │   │   └── score.rs
│   │   │   ├── systems/           # Game-specific systems
│   │   │   │   ├── mod.rs
│   │   │   │   ├── player_control.rs
│   │   │   │   ├── movement.rs
│   │   │   │   ├── collision.rs
│   │   │   │   ├── asteroid_spawn.rs
│   │   │   │   ├── bullet_lifecycle.rs
│   │   │   │   ├── scoring.rs
│   │   │   │   └── cleanup.rs
│   │   │   ├── assets/            # Game asset definitions
│   │   │   │   ├── mod.rs
│   │   │   │   ├── ship.rs
│   │   │   │   ├── asteroid.rs
│   │   │   │   ├── bullet.rs
│   │   │   │   └── effects.rs
│   │   │   ├── ui/                # Game UI
│   │   │   │   ├── mod.rs
│   │   │   │   ├── main_menu.rs
│   │   │   │   ├── hud.rs
│   │   │   │   ├── pause_menu.rs
│   │   │   │   └── high_scores.rs
│   │   │   ├── audio/             # Game audio management
│   │   │   │   ├── mod.rs
│   │   │   │   ├── manager.rs
│   │   │   │   └── effects.rs
│   │   │   └── gameplay/          # Core gameplay logic
│   │   │       ├── mod.rs
│   │   │       ├── difficulty.rs
│   │   │       ├── wave_manager.rs
│   │   │       ├── power_ups.rs
│   │   │       └── scoring.rs
│   │   └── tests/                 # Game tests
│   │       ├── integration/
│   │       └── unit/
│   └── engine_plugins/            # Optional engine plugins
│       ├── debug_renderer/        # Debug visualization plugin
│       │   ├── Cargo.toml
│       │   └── src/
│       ├── imgui_integration/     # ImGui plugin
│       │   ├── Cargo.toml
│       │   └── src/
│       ├── console/               # In-game console plugin
│       │   ├── Cargo.toml
│       │   └── src/
│       └── profiler/              # Performance profiler plugin
│           ├── Cargo.toml
│           └── src/
├── tools/                         # Development tools
│   ├── asset_processor/           # Asset pipeline tool
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── shader_compiler/           # Shader compilation tool
│   │   ├── Cargo.toml
│   │   └── src/
│   └── model_converter/           # Model format converter
│       ├── Cargo.toml
│       └── src/
├── target/                        # Build output (unchanged)
└── .github/                       # GitHub configuration
    ├── workflows/
    │   ├── ci.yml
    │   └── release.yml
    └── copilot-instructions.md
```

## Workspace Configuration

### Root Cargo.toml
```toml
[workspace]
members = [
    "crates/rust_engine",
    "crates/asteroids",
    "crates/engine_plugins/debug_renderer",
    "crates/engine_plugins/imgui_integration",
    "crates/engine_plugins/console",
    "crates/engine_plugins/profiler",
    "tools/asset_processor",
    "tools/shader_compiler",
    "tools/model_converter",
]

[workspace.dependencies]
# Shared dependencies for all crates
ash = "0.37"
glfw = "0.54"
winit = "0.28"
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
anyhow = "1.0"
log = "0.4"
env_logger = "0.10"

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
```

### Engine Crate Cargo.toml
```toml
[package]
name = "rust_engine"
version = "0.1.0"
edition = "2021"
description = "A modular game engine written in Rust"
license = "MIT OR Apache-2.0"

[dependencies]
# Core dependencies
ash = { workspace = true }
glfw = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
log = { workspace = true }

# Math
nalgebra = "0.32"
approx = "0.5"

# Memory management
vk-mem = "0.3"

# Asset loading
image = "0.24"
tobj = "3.2"

# Audio
rodio = "0.17"

[features]
default = ["vulkan", "audio", "physics"]
vulkan = []
audio = ["rodio"]
physics = []
debug = []
hot_reload = []

[dev-dependencies]
criterion = "0.5"
proptest = "1.0"
```

### Asteroids Game Cargo.toml
```toml
[package]
name = "asteroids"
version = "0.1.0"
edition = "2021"
description = "Asteroids game built with RustEngine"
license = "MIT OR Apache-2.0"

[dependencies]
rust_engine = { path = "../rust_engine" }
serde = { workspace = true }
anyhow = { workspace = true }
log = { workspace = true }

# Game-specific dependencies
rand = "0.8"
ron = "0.8"  # For configuration files

[features]
default = []
debug = ["rust_engine/debug"]
```

## Migration Strategy

### Phase 1: Create New Structure
1. Create the new directory structure
2. Set up workspace configuration
3. Create placeholder Cargo.toml files
4. Move existing files to appropriate locations

### Phase 2: Engine Foundation
1. Implement core engine traits and interfaces
2. Move generic code from current src/ to engine crate
3. Set up basic engine initialization
4. Create application trait

### Phase 3: Application Layer
1. Implement AsteroidsApp using engine traits
2. Move game-specific code to asteroids crate
3. Set up asset loading for game assets
4. Implement basic game loop

### Phase 4: Integration and Testing
1. Ensure all tests pass
2. Verify engine-application separation
3. Add integration tests
4. Update documentation

## Benefits of This Structure

### Code Organization
- **Clear Separation**: Engine code completely separate from game code
- **Reusability**: Engine can be used for other projects
- **Modularity**: Each crate has focused responsibility
- **Scalability**: Easy to add new games or engine features

### Development Benefits
- **Independent Testing**: Test engine and game separately
- **Parallel Development**: Multiple developers can work on different crates
- **Faster Compilation**: Only rebuild changed crates
- **Better Documentation**: Separate docs for engine API and game

### Maintenance Benefits
- **Clear Interfaces**: Trait-based communication between layers
- **Isolated Changes**: Engine changes don't affect game logic (if API stable)
- **Version Management**: Can version engine separately from games
- **Plugin System**: Easy to add optional features

## Implementation Notes

### File Movement Mapping
```
Current → New Location
src/util/ → crates/rust_engine/src/foundation/
src/ecs/ → crates/rust_engine/src/ecs/
src/render/ → crates/rust_engine/src/render/
src/input/ → crates/rust_engine/src/input/
src/audio/ → crates/rust_engine/src/audio/
src/assets/ → crates/rust_engine/src/assets/
src/config/ → Split between engine config and game config
src/game/ → crates/asteroids/src/components/ and systems/
src/menu/ → crates/asteroids/src/ui/
src/save/ → crates/asteroids/src/ (game-specific)
src/application.rs → crates/rust_engine/src/application.rs (trait)
src/main.rs → crates/asteroids/src/main.rs
```

### API Design Principles
1. **Trait-based**: All engine-application communication through traits
2. **Handle-based**: Resources accessed via type-safe handles
3. **Error Propagation**: Proper error handling throughout
4. **Zero-cost Abstractions**: No runtime overhead for abstractions
5. **Builder Patterns**: Fluent APIs for complex object creation

This restructure provides a solid foundation for building both a reusable engine and the Asteroids game while maintaining clean architecture and enabling future growth.
