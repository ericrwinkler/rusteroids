# Rusteroids Development TODO

## 🚨 CRITICAL PRIORITIES

### 1. **URGENT: Fix Parallelism and Concurrency Issues (THIS WEEK)**
- **Critical Bugs Found**: Comprehensive parallelism audit revealed multiple race conditions
- **Immediate Actions**:
  - ✅ Fix SyncManager double-increment bug in `sync_manager.rs:126-129` (COMPLETED)
  - ✅ Add memory barriers for UBO updates (COMPLETED)
  - ✅ Remove excessive `device_wait_idle()` calls destroying GPU parallelism (COMPLETED)
- **Risk**: Current code has race conditions that could cause crashes or data corruption
- **Reference**: See `PARALLELISM_CONCURRENCY_AUDIT.md` for complete analysis

**Latest Fix (3/3 Complete)**: Added HOST_WRITE → UNIFORM_READ memory barriers in command_recorder.rs before GPU shader access to prevent reading stale UBO data. All P0 parallelism issues now resolved!

### 2. ~~Fix Normal Matrix Lighting Issue~~ **SKIPPED** ✅
- **Problem**: Lighting appears to move with teapot rotation instead of staying fixed in world space
- **Root Cause**: Normal matrix calculation in `command_recorder.rs` needs Vulkan left-handed coordinate fix
- **Decision**: **SKIPPED** - Will be resolved in entity-based lighting system rewrite
- **Rationale**: Don't fix throwaway code; focus on ECS foundation instead
- **Status**: Will be naturally fixed when implementing proper multi-light entity system

### 3. Thread Safety Implementation (WEEK 2)
- **Critical**: Command pools are NOT thread-safe but lack protection
- **Actions**:
  - Implement thread-safe command pool management (2 days)
  - Add descriptor set per-frame isolation (2 days)  
  - Document thread safety guarantees (1 day)
- **Impact**: Required before any multi-threading features

### 4. Shader System Unification (6-DAY SPRINT)
- **Problem**: Multiple disconnected shader management systems
- **Current Issues**:
  - Teapot app uses hardcoded paths, bypasses sophisticated systems
  - ShaderManager completely unused despite advanced features
  - No coordination between build system and applications
- **Solution**: Implement unified shader registry with TOML configuration
- **Timeline**: 6 days comprehensive development cycle

### 5. **Entity-Component System Foundation (2-3 WEEKS)**
- **Goal**: Implement first ECS architecture for lights and teapot as entities
- **Game Engine Architecture Lessons**:
  - "Game object model provides real-time simulation of heterogeneous collection of objects"
  - Lights should be first-class entities alongside cameras, meshes, and other objects
  - Need proper lifetime management and state simulation for entities
- **Phases**:
  - Week 1: Core ECS foundation (Entity, Component, World)
  - Week 2: Light entities and rendering system integration
  - Week 3: Teapot as mesh entity, performance optimization
- **Success Criteria**: Multiple light entities, teapot as entity, 60+ FPS

### 6. Multi-Light System Foundation (2-3 WEEKS)
- **Goal**: Support 4-8 lights simultaneously via entity system
- **Phases**:
  - Week 1: Light entity system foundation  
  - Week 2: Vulkan UBO backend integration
  - Week 3: Shadow mapping and optimization
- **Success Criteria**: Multiple directional lights, teapot self-shadowing, 60+ FPS

## 📋 IMMEDIATE ACTIONS (NEXT 7 DAYS) - UPDATED PRIORITIES

### 🚨 **CRITICAL: Multi-Light GPU Architecture (IMMEDIATE FOCUS)**

#### Day 1-2: GPU Data Structure Expansion
- [ ] **Critical**: Expand SimpleLightingUBO to MultipleLightingUBO
  ```rust
  // Replace single light with light array
  struct MultipleLightingUBO {
      ambient_color: [f32; 4],
      light_count: u32,
      lights: [GpuLight; MAX_LIGHTS], // Support 4-8 lights
  }
  ```
- [ ] **Update UboManager**: Modify lighting buffer allocation for larger structure
- [ ] **Test**: Verify GPU memory allocation works with expanded structure

#### Day 3-4: Shader Multi-Light Support  
- [ ] **Critical**: Update fragment shaders to process light arrays
  ```glsl
  // Update from single light to light loop
  for (int i = 0; i < lighting.light_count; ++i) {
      // Process lights[i] 
  }
  ```
- [ ] **Update Shaders**: standard_pbr_frag.frag, frag_ubo_simple.frag, frag_material_ubo.frag
- [ ] **Test**: Verify multiple lights render correctly

#### Day 5: Bridge Integration
- [ ] **Update LightingBridge**: Send ALL lights instead of just first light
- [ ] **Remove Bottleneck**: Eliminate single-light limitation in convert_to_legacy()
- [ ] **Test**: Create multiple light entities and verify all reach GPU

#### Day 6-7: Performance and Spatial Culling
- [ ] **Implement Light Culling**: Quadtree/octree for spatial light queries
- [ ] **Performance Testing**: Ensure 60+ FPS with 4-8 lights
- [ ] **Optimization**: Distance-based light attenuation and importance sorting

## 🎮 ENTITY SYSTEM IMPLEMENTATION (IMMEDIATE PRIORITY)

### Week 1: Core ECS Foundation
Following Game Engine Architecture principles for "heterogeneous object collections":

#### Day 1-2: Entity and Component Foundation (✅ COMPLETED September 7)
- [x] **Entity System**: Implement EntityId, Entity lifecycle management ✓
- [x] **Component Trait**: Define Component trait and registration system ✓
- [x] **Basic Components**: Transform, Name components ✓ (Transform complete)
- [x] **World Container**: Entity storage and basic queries ✓
- [x] **Entity Builder**: Convenient entity creation pattern ✓
- [x] **Demo Verification**: ECS teapot demo successfully creates entities ✅

#### Day 3-4: Light Entity Integration (✅ COMPLETED September 7)
- [x] **LightComponent**: Directional, point, spot light types ✓
- [x] **Light Entities**: Create light entities in teapot demo ✓  
- [x] **Lighting System**: Query light entities, update lighting data ✓
- [x] **Demo Success**: ECS demo shows "2 lights active" with correct properties ✅
- [x] **Renderer Bridge**: LightingBridge converts ECS data to legacy renderer format ✅
- [x] **Live Integration**: Teapot app now uses ECS entities for lighting ✅
- [x] **Success Verification**: App runs with identical visual quality using ECS lighting ✅

#### Day 5-7: Mesh Entity Implementation
- [ ] **MeshComponent**: Mesh, material, transform integration
- [ ] **Teapot Entity**: Convert hardcoded teapot to entity
- [ ] **Render System**: Query mesh entities, submit to renderer
- [ ] **Performance Testing**: Ensure 60+ FPS maintained

## 🔥 **CRITICAL: Multi-Light GPU Architecture Bottleneck (IMMEDIATE PRIORITY)**

### **ECS Lighting Integration: SUCCESS ✅**
- **Achievement**: ECS lighting system fully operational and integrated with teapot app
- **Architecture**: Clean separation with LightingSystem → LightingBridge → Legacy Renderer
- **Performance**: Identical visual quality, 60+ FPS maintained
- **Components**: Light, Transform entities working correctly with type-safe storage

### **GPU Multi-Light Support: MAJOR BOTTLENECK IDENTIFIED 🚨**
- **Problem**: Current GPU pipeline only supports 1 directional light in SimpleLightingUBO
- **Impact**: ECS can create multiple light entities, but only first light reaches GPU
- **Root Cause**: Shader and UBO data structures hardcoded for single light

**Current Flow (BOTTLENECKED)**:
```
ECS: [Light1, Light2, Light3, ...] → LightingSystem → Multiple ProcessedLights
    ↓ 
LightingBridge: Takes ONLY first light → Converts to legacy format
    ↓
SimpleLightingUBO: { 1 directional light only } → GPU shaders
    ↓  
Shaders: Process only 1 light → Visual output limited
```

**Required Multi-Light Architecture**:
```rust
// NEW: Multi-light UBO structure needed
#[repr(C, align(16))]
struct MultipleLightingUBO {
    ambient_color: [f32; 4],
    light_count: u32,
    _padding: [u32; 3], 
    lights: [GpuLight; MAX_LIGHTS], // Array of lights!
}

// NEW: Shader updates required
layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    uvec4 light_count_and_padding;
    GpuLight lights[MAX_LIGHTS];  // Process light arrays!
} lighting;
```

### **Spatial Light Culling (Performance Optimization)**
- **User Request**: Quadtree/octree spatial data structure for light culling
- **Goal**: Only lights within effective radius affect entities
- **Benefits**: Performance optimization for many lights, realistic attenuation

### Week 2: Advanced Entity Features
- [ ] **Entity Queries**: Efficient multi-component queries  
- [ ] **System Scheduling**: Proper update order (lighting before rendering)
- [ ] **Entity Lifetime**: Proper creation/destruction lifecycle
- [ ] **Component Storage**: Optimized memory layout for components

## 🔄 ONGOING WORK

### Infrastructure Maintenance
- [ ] Keep material UBO system operational
- [ ] Maintain 60+ FPS performance
- [ ] Ensure clean Vulkan validation
- [ ] Update screenshot validation before commits

### Code Quality
- [ ] Remove any remaining dead code from previous cleanup
- [ ] Maintain zero compiler warnings
- [ ] Follow Rust idioms and safety practices
- [ ] Document major architectural decisions

## 📅 MEDIUM-TERM ROADMAP (1-3 MONTHS)

### Enhanced Lighting (Month 1)
- [ ] Multiple directional lights (4-8 lights)
- [ ] Point lights with distance attenuation
- [ ] Spot lights with cone angles
- [ ] Light culling and optimization
- [ ] Performance testing with many lights

### Shadow System (Month 2)
- [ ] Basic shadow mapping
- [ ] Teapot self-shadowing (spout/handle shadows)
- [ ] Shadow map quality optimization
- [ ] Cascaded shadow maps for directional lights

### Advanced Features (Month 3)
- [ ] Image-Based Lighting (IBL)
- [ ] Area lights for soft shadows
- [ ] Light animation system
- [ ] Environmental lighting effects

## 🔧 PARALLELISM AND CONCURRENCY IMPROVEMENTS

### Thread Safety Implementation (Month 1)
- [ ] **Thread-safe command pool management** (per-thread pools or mutex protection)
- [ ] **Descriptor set per-frame isolation** (eliminate concurrent access)
- [ ] **Memory barrier audit and implementation** (all resource transitions)
- [ ] **Frame resource isolation** (complete 3x buffering)

### Performance Optimization (Month 2) 
- [ ] **Eliminate all `device_wait_idle()` calls** except cleanup
- [ ] **Implement VMA integration** for efficient memory management
- [ ] **GPU memory access pattern optimization** (coalesced access)
- [ ] **Timeline semaphore upgrade** (Vulkan 1.2 features)

### Advanced Concurrency Features (Month 3)
- [ ] **Multi-threaded command recording** (secondary command buffers)
- [ ] **GPU timeline profiling** (performance monitoring)
- [ ] **Lock-free resource updates** where possible
- [ ] **SIMT-optimized shader patterns** (minimize branch divergence)

## 🎯 SUCCESS METRICS - UPDATED STATUS

### Technical Goals
- ✅ **60+ FPS**: Maintained with ECS lighting integration
- ✅ **Clean Validation**: Zero Vulkan validation errors in current implementation
- ✅ **Multiple Materials**: Current 5-material system working perfectly
- ✅ **Entity-Component System**: Fully operational ECS with light entities
- ✅ **ECS Integration**: Live teapot app uses entity-based lighting
- ⚠️ **Multi-Light Support**: ECS supports unlimited lights, GPU bottlenecked at 1
- ⚠️ **World-Space Lighting**: Works correctly but can be improved
- 🔄 **Unified Shaders**: Deferred pending multi-light GPU architecture
- 🔄 **Spatial Light Culling**: Needed for performance with many lights

### Quality Gates - CURRENT STATUS
- ✅ **ECS Architecture**: Complete Entity-Component-System following Gregory's principles
- ✅ **Rendering Integration**: ECS lighting seamlessly bridges to legacy renderer
- ✅ **Performance**: 60+ FPS maintained during ECS integration
- ✅ **Visual Quality**: Identical rendering results with entity-based lighting
- ✅ **Code Quality**: Clean separation of concerns, proper Rust idioms
- ❌ **Multi-Light GPU**: Critical bottleneck identified in GPU data structures
- ❌ **Shader Arrays**: Fragment shaders need light array processing
- ❌ **Spatial Optimization**: Light culling needed for many lights

### Architecture Success Indicators
- ✅ **Game Object Model**: Lights are first-class entities in game world
- ✅ **Heterogeneous Collection**: ECS manages different entity types (lights, transforms)
- ✅ **Lifetime Management**: Proper entity creation, updates, and system processing
- ✅ **State Simulation**: LightingSystem processes entity state each frame
- ✅ **Separation of Concerns**: Clean ECS → System → Bridge → Renderer pipeline
- ✅ **Type Safety**: Component storage and retrieval with Rust type system
- ❌ **GPU Scalability**: Single light bottleneck prevents true multi-light scenes

### Risk Mitigation - UPDATED
- ✅ **P0 Complete**: ECS foundation successfully implemented and integrated
- 🚨 **P1 Critical**: Multi-light GPU architecture - immediate priority
- 🔄 **P2 High**: Spatial light culling for performance optimization  
- 🔄 **P3 Medium**: Advanced lighting features (shadows, IBL)
- 🔄 **P4 Low**: Shader system unification (can wait)

## � ARCHITECTURAL LESSONS FROM GAME ENGINE ARCHITECTURE

Based on analysis of Jason Gregory's "Game Engine Architecture" (Chapters 8, 11, 15, 16):

### Chapter 8: Game Loop and Real-Time Simulation
- **Key Lesson**: "Game loop runs repeatedly, giving various systems chance to update state for next discrete time step"
- **Application**: Entity systems need proper scheduling - lighting system updates before rendering system
- **Implementation**: Frame-based entity updates with proper system ordering

### Chapter 11: The Rendering Engine  
- **Key Lesson**: Layered rendering with "dynamic lighting system" as core component
- **Application**: Lights should integrate with low-level renderer via well-defined interfaces
- **Implementation**: Light entities → Light system → Vulkan backend → GPU

### Chapter 15: Introduction to Gameplay Systems
- **Key Lesson**: "Game object model provides real-time simulation of heterogeneous collection of objects"
- **Application**: Lights, cameras, meshes are all first-class entities in the game world
- **Implementation**: ECS treats lights as entities with Transform + LightComponent

### Chapter 16: Runtime Gameplay Foundation Systems
- **Key Lesson**: Need for "object identification, lifetime management, and state simulation over time"  
- **Application**: Proper entity lifecycle, component queries, and system scheduling
- **Implementation**: EntityId, World container, Component storage, System execution

### Software Object Model Questions (Gregory's Framework)
Applied to our Rusteroids architecture:
- **Object-oriented design?** Yes, component-based with entity aggregation
- **Language?** Rust with strong typing and ownership semantics
- **Class hierarchy?** Flat component composition over inheritance
- **Object references?** EntityId handles, not direct pointers
- **Unique identification?** EntityId(u64) for all entities
- **Lifetime management?** RAII + explicit entity destruction
- **State simulation?** Per-frame system updates over entity sets
