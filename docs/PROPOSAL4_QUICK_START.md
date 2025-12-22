# Proposal #4 + Transparency - Quick Start Guide

**Date**: December 22, 2025  
**Full Plan**: See `IMPLEMENTATION_PLAN_PROPOSAL4_TRANSPARENCY.md`

---

## What We're Building

### Problem
- ❌ Application manually creates 9 mesh pools
- ❌ Application manually allocates 3 instances
- ❌ `is_transparent` flag broken (objects disappear)
- ❌ No particle alpha fade
- ❌ No reference counting
- ❌ No automatic pool growth

### Solution
- ✅ Resource Manager handles all pooling automatically
- ✅ Transparent rendering pass implemented
- ✅ Single API: `scene_manager.create_renderable_entity()`
- ✅ Automatic reference counting
- ✅ Automatic pool growth
- ✅ Particle alpha fade works

---

## 30-Second Architecture

```
Application
    ↓ create_renderable_entity()
Scene Manager
    ↓ request_mesh_instance()
Resource Manager (NEW)
    ├─ Cache: Same mesh+materials → reuse pool
    ├─ Ref Count: Track usage, auto-unload
    ├─ Growth: Pools grow automatically
    └─ Transparency: Separate opaque/transparent passes
    ↓
MeshPoolManager (Reuse existing)
    ↓
Vulkan Renderer
```

---

## Key Files to Create/Modify

### Create New
1. `crates/rust_engine/src/assets/resource_manager.rs` - Core Resource Manager
2. `crates/rust_engine/src/assets/mod.rs` - Module export

### Modify Existing
1. `crates/rust_engine/src/render/mod.rs` - Add transparent pass
2. `crates/rust_engine/src/scene/scene_manager.rs` - Integrate Resource Manager
3. `teapot_app/src/main_dynamic.rs` - Remove 9 manual pool calls
4. `teapot_app/src/main_fleet.rs` - Remove 3 manual pool calls

---

## 4-Week Timeline

### Week 1: Foundation
**Goal**: Resource Manager core functionality

```rust
// Create this
pub struct ResourceManager {
    pool_cache: HashMap<PoolKey, MeshPoolManager>,
    mesh_refs: HashMap<MeshId, usize>,
    // ...
}

impl ResourceManager {
    pub fn request_mesh_instance(...) -> Handle { }
    pub fn release_mesh_instance(...) { }
}
```

**Deliverable**: Resource Manager compiles, passes unit tests

---

### Week 2: Transparency Flag Fix
**Goal**: Fix the broken `is_transparent` flag (rendering already works!)

**IMPORTANT**: Transparent rendering is ALREADY IMPLEMENTED in `pool_manager.rs`!
- ✅ Global depth sorting back-to-front
- ✅ Correct pipeline selection via Material.required_pipeline()
- ✅ Two-pass rendering (opaque first, transparent second)

**What's Broken**: The `is_transparent` flag is hardcoded to `false` as a workaround.

**The Fix**:
```rust
// scene_manager.rs - Detect transparency from Material
let is_transparent = matches!(
    material.required_pipeline(),
    PipelineType::TransparentPBR | PipelineType::TransparentUnlit
);

let renderable = if is_transparent {
    RenderableComponent::new_transparent(material, mesh, mesh_type, 0)
} else {
    RenderableComponent::new(material, mesh, mesh_type)
};
```

**Deliverable**: Flag correctly reflects Material transparency, FIXME comments removed

---

### Week 3: Integration + Immediate Cleanup
**Goal**: Migrate apps and DELETE deprecated functions ASAP

**Migration** (both main_dynamic.rs and main_fleet.rs):
```rust
// BEFORE: Manual pool creation (DELETE THIS)
self.graphics_engine.create_mesh_pool(MeshType::Sphere, mesh, materials, 50)?;
let handle = self.graphics_engine.allocate_from_pool(MeshType::Sphere, transform, material)?;

// AFTER: Automatic pooling
let entity = self.scene_manager.create_renderable_entity(
    &mut world, mesh, MeshType::Sphere, material, transform
)?;
```

**Immediate Deletion** (Day 3):
- DELETE `create_mesh_pool()` from render/mod.rs and graphics_api.rs
- DELETE `allocate_from_pool()` from both files
- DELETE `free_pool_instance()` from both files
- DELETE `update_pool_instance()` from both files
- KEEP `update_instance_material()` (new function for alpha fade)

**Verification**:
```bash
grep -r "create_mesh_pool\|allocate_from_pool" crates/  # Should be 0 matches
```

**Deliverable**: Zero manual pool management, deprecated functions DELETED

---

### Week 4: Polish
**Goal**: Automatic pool growth + benchmarks

```rust
impl ResourceManager {
    fn allocate_or_grow(...) {
        match pool.allocate_instance(...) {
            Ok(handle) => Ok(handle),
            Err(PoolExhausted) => {
                self.grow_pool()?;  // Automatic growth
                pool.allocate_instance(...)
            }
        }
    }
}
```

**Deliverable**: Pools grow automatically, performance benchmarks pass

---

## How to Start

### Step 1: Create Resource Manager Module
```bash
cd crates/rust_engine/src
mkdir -p assets
touch assets/mod.rs
touch assets/resource_manager.rs
```

### Step 2: Add to lib.rs
```rust
// crates/rust_engine/src/lib.rs
pub mod assets;
```

### Step 3: Start Coding
Follow **Phase 1** in full implementation plan:
- Create ResourceManager struct
- Implement request_mesh_instance()
- Implement release_mesh_instance()
- Write unit tests

---

## Quick Reference: API Changes

### Old API (Deprecated)
```rust
// Manual pool creation
graphics_engine.create_mesh_pool(MeshType::Teapot, mesh, materials, 100)?;

// Manual allocation
let handle = graphics_engine.allocate_from_pool(MeshType::Teapot, transform, material)?;

// Manual cleanup
graphics_engine.free_pool_instance(MeshType::Teapot, handle)?;
```

### New API (Proposal #4)
```rust
// Everything automatic
let entity = scene_manager.create_renderable_entity(
    &mut world,
    mesh,
    MeshType::Teapot,
    material,
    transform,
)?;

// Cleanup automatic on entity destroy
scene_manager.destroy_entity(&mut world, entity)?;
```

---

## Transparent Rendering Changes

### Old Behavior
```rust
// Setting is_transparent=true breaks rendering
let renderable = RenderableComponent::new_transparent(material, mesh, mesh_type, 0);
// ❌ Object disappears (moved to unrendered batch)
```

### New Behavior
```rust
// Setting is_transparent=true works correctly
let renderable = RenderableComponent::new_transparent(material, mesh, mesh_type, 0);
// ✅ Object renders in transparent pass, sorted back-to-front
```

---

## Testing Strategy

### Run After Each Week
```bash
# Unit tests
cargo test --lib resource_manager

# Integration tests  
cargo test --test scene_integration

# Visual test
cargo run --bin teapot_dynamic --release
cargo run --bin fleet_demo --release

# Benchmarks (Week 4)
cargo bench --bench resource_manager
```

---

## Success Criteria

**After Week 1**:
- [ ] `cargo build` succeeds
- [ ] ResourceManager unit tests pass
- [ ] Can create/destroy pools programmatically

**After Week 2**:
- [ ] Transparent monkeys render correctly
- [ ] Can set `is_transparent=true` without breakage
- [ ] All FIXME comments removed

**After Week 3**:
- [ ] `main_dynamic.rs` has zero `create_mesh_pool()` calls
- [ ] `main_fleet.rs` has zero `allocate_from_pool()` calls
- [ ] Application runs identically to before

**After Week 4**:
- [ ] Pools grow automatically (no size specified)
- [ ] Performance benchmarks pass
- [ ] Memory usage <500MB for 1000 objects

---

## Rollback Plan

If major issues occur:

1. **Week 1**: Just delete `assets/` folder (no application changes yet)
2. **Week 2**: Revert render/mod.rs changes (keep opaque-only rendering)
3. **Week 3**: Keep Resource Manager, don't migrate application yet
4. **Week 4**: Disable auto-growth (use fixed pool sizes)

---

## Help & Resources

- **Full Plan**: `IMPLEMENTATION_PLAN_PROPOSAL4_TRANSPARENCY.md`
- **API Spec**: `API_REFACTORING_PLAN.md` (Proposal #4)
- **Book**: Game Engine Architecture 3rd Ed, Chapters 6.2 & 7.2
- **Current Code**: `render/systems/dynamic/` (existing pool system to reuse)

---

## Questions to Answer Before Starting

1. **Memory Budget**: What's max memory for pools? (Suggest: 512MB)
2. **Initial Pool Size**: Start small or large? (Suggest: 16)
3. **Growth Factor**: How fast to grow? (Suggest: 1.5x)
4. **Max Pool Size**: Hard limit? (Suggest: 10,000)

Defaults in `ResourceConfig`:
```rust
pub struct ResourceConfig {
    pub memory_budget: usize,      // 512 MB
    pub initial_pool_size: usize,  // 16
    pub growth_factor: f32,         // 1.5
    pub max_pool_size: usize,       // 10000
}
```

---

**Ready to start?** Read full plan → Create Git branch → Begin Week 1 🚀
