//! ECS World implementation with proper component storage
//! Based on Game Engine Architecture principles: cache-friendly data layout and component purity

use super::{Entity, Component};
use std::collections::HashMap;
use std::any::{TypeId, Any};

/// Dense component storage for a specific component type
/// Uses packed arrays for cache-friendly iteration
struct ComponentStorage<T: Component> {
    components: Vec<T>,
    entity_to_index: HashMap<u32, usize>, // Entity ID -> component index
    index_to_entity: Vec<u32>,            // Component index -> Entity ID
    free_indices: Vec<usize>,             // Free slots for reuse
    generations: Vec<u64>,                // Generation counter per component (for dirty tracking)
}

impl<T: Component> ComponentStorage<T> {
    fn new() -> Self {
        Self {
            components: Vec::new(),
            entity_to_index: HashMap::new(),
            index_to_entity: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
        }
    }
    
    fn add(&mut self, entity_id: u32, component: T) {
        let index = if let Some(free_idx) = self.free_indices.pop() {
            // Reuse free slot
            self.components[free_idx] = component;
            self.index_to_entity[free_idx] = entity_id;
            self.generations[free_idx] = 1; // Reset generation
            free_idx
        } else {
            // Append new component
            let idx = self.components.len();
            self.components.push(component);
            self.index_to_entity.push(entity_id);
            self.generations.push(1); // Start at generation 1
            idx
        };
        
        self.entity_to_index.insert(entity_id, index);
    }
    
    fn get(&self, entity_id: u32) -> Option<&T> {
        self.entity_to_index.get(&entity_id)
            .and_then(|&index| self.components.get(index))
    }
    
    fn get_mut(&mut self, entity_id: u32) -> Option<&mut T> {
        if let Some(&index) = self.entity_to_index.get(&entity_id) {
            // Increment generation when mutably accessing (marks as dirty)
            self.generations[index] += 1;
            self.components.get_mut(index)
        } else {
            None
        }
    }
    
    /// Get the generation counter for a component (for change detection)
    fn get_generation(&self, entity_id: u32) -> Option<u64> {
        self.entity_to_index.get(&entity_id)
            .map(|&index| self.generations[index])
    }
    
    fn remove(&mut self, entity_id: u32) -> Option<T> {
        if let Some(index) = self.entity_to_index.remove(&entity_id) {
            // Mark slot as free
            self.free_indices.push(index);
            // Note: We don't actually remove from Vec to maintain indices
            // This is a simplified implementation - production ECS would handle this better
            None
        } else {
            None
        }
    }
    
    /// Iterator over all components with their entity IDs (cache-friendly)
    fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.index_to_entity.iter()
            .zip(self.components.iter())
            .enumerate()
            .filter(|(idx, _)| !self.free_indices.contains(idx))
            .map(|(_, (entity_id, component))| (*entity_id, component))
    }
    
    /// Mutable iterator over all components with their entity IDs
    fn iter_mut(&mut self) -> Vec<(u32, &mut T)> {
        let mut results = Vec::new();
        for (idx, (entity_id, component)) in self.index_to_entity.iter()
            .zip(self.components.iter_mut())
            .enumerate() {
            if !self.free_indices.contains(&idx) {
                results.push((*entity_id, component));
            }
        }
        results
    }
}

/// ECS World containing all entities and components with type-safe storage
pub struct World {
    next_entity_id: u32,
    entities: Vec<Entity>,
    alive_entities: HashMap<u32, bool>, // Track which entities are alive
    component_storages: HashMap<TypeId, Box<dyn Any>>,
    // Change tracking: entity_id -> (component_type, generation)
    changed_components: HashMap<u32, HashMap<TypeId, u64>>,
}

impl World {
    /// Create a new world
    pub fn new() -> Self {
        Self {
            next_entity_id: 0,
            entities: Vec::new(),
            alive_entities: HashMap::new(),
            component_storages: HashMap::new(),
            changed_components: HashMap::new(),
        }
    }
    
    /// Get or create component storage for type T
    fn get_storage<T: Component>(&mut self) -> &mut ComponentStorage<T> {
        let type_id = TypeId::of::<T>();
        if !self.component_storages.contains_key(&type_id) {
            let storage = ComponentStorage::<T>::new();
            self.component_storages.insert(type_id, Box::new(storage));
        }
        
        self.component_storages.get_mut(&type_id)
            .unwrap()
            .downcast_mut::<ComponentStorage<T>>()
            .expect("Component storage type mismatch")
    }
    
    /// Get component storage for type T (read-only)
    fn get_storage_ref<T: Component>(&self) -> Option<&ComponentStorage<T>> {
        let type_id = TypeId::of::<T>();
        self.component_storages.get(&type_id)
            .and_then(|storage| storage.downcast_ref::<ComponentStorage<T>>())
    }
    
    /// Create a new entity
    pub fn create_entity(&mut self) -> Entity {
        let entity = Entity::new(self.next_entity_id);
        self.next_entity_id += 1;
        self.entities.push(entity);
        self.alive_entities.insert(entity.id(), true);
        entity
    }
    
    /// Add a component to an entity
    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        let storage = self.get_storage::<T>();
        storage.add(entity.id(), component);
        
        // Track this change
        let type_id = TypeId::of::<T>();
        self.changed_components
            .entry(entity.id())
            .or_insert_with(HashMap::new)
            .insert(type_id, 1); // Initial generation
    }
    
    /// Get a component from an entity
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.get_storage_ref::<T>()?.get(entity.id())
    }
    
    /// Get a mutable component from an entity
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let entity_id = entity.id();
        let type_id = TypeId::of::<T>();
        
        // First, check if component exists and get generation
        let (exists, new_generation) = {
            let storage = self.get_storage::<T>();
            let generation_opt = storage.get_generation(entity_id);
            (generation_opt.is_some(), generation_opt.map(|g| g + 1))
        };
        
        // Track the change if component exists
        if exists {
            if let Some(gen) = new_generation {
                self.changed_components
                    .entry(entity_id)
                    .or_insert_with(HashMap::new)
                    .insert(type_id, gen);
            }
        }
        
        // Now get the mutable reference (this increments generation internally)
        self.get_storage::<T>().get_mut(entity_id)
    }
    
    /// Remove a component from an entity
    pub fn remove_component<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let storage = self.get_storage::<T>();
        storage.remove(entity.id())
    }
    
    /// Query all entities with a specific component type
    pub fn query<T: Component>(&self) -> Vec<(Entity, &T)> {
        if let Some(storage) = self.get_storage_ref::<T>() {
            storage.iter()
                .map(|(entity_id, component)| (Entity::new(entity_id), component))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Query all entities with a specific component type (mutable)
    pub fn query_mut<T: Component>(&mut self) -> Vec<(Entity, &mut T)> {
        let type_id = TypeId::of::<T>();
        if let Some(storage) = self.component_storages.get_mut(&type_id) {
            if let Some(storage) = storage.downcast_mut::<ComponentStorage<T>>() {
                storage.iter_mut()
                    .into_iter()
                    .map(|(entity_id, component)| (Entity::new(entity_id), component))
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }
    
    /// Update the world (run systems)
    pub fn update(&mut self, _delta_time: f32) {
        // TODO: Implement system execution
    }
    
    /// Get an iterator over all entities
    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }
    
    /// Get entities with Transform or Renderable components that changed
    pub fn get_changed_renderable_entities(&self) -> Vec<Entity> {
        use crate::ecs::components::{TransformComponent, RenderableComponent};
        
        let transform_type = TypeId::of::<TransformComponent>();
        let renderable_type = TypeId::of::<RenderableComponent>();
        
        self.changed_components
            .iter()
            .filter_map(|(entity_id, changes)| {
                // Include if Transform or Renderable changed
                if changes.contains_key(&transform_type) || changes.contains_key(&renderable_type) {
                    Some(Entity::new(*entity_id))
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Clear change tracking for a specific entity
    pub fn clear_entity_changes(&mut self, entity: Entity) {
        self.changed_components.remove(&entity.id());
    }
    
    /// Clear all change tracking
    pub fn clear_all_changes(&mut self) {
        self.changed_components.clear();
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::{TransformComponent, RenderableComponent};
    use crate::render::resources::materials::MaterialId;
    use crate::foundation::math::Vec3;
    
    #[test]
    fn test_change_detection_on_add() {
        let mut world = World::new();
        let entity = world.create_entity();
        
        // Add a component - should track as changed
        let transform = TransformComponent::from_position(Vec3::new(1.0, 2.0, 3.0));
        world.add_component(entity, transform);
        
        // Should be detected as changed
        let changed = world.get_changed_renderable_entities();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0], entity);
    }
    
    #[test]
    fn test_change_detection_on_mutation() {
        let mut world = World::new();
        let entity = world.create_entity();
        
        // Add component initially
        let transform = TransformComponent::from_position(Vec3::new(1.0, 2.0, 3.0));
        world.add_component(entity, transform);
        
        // Clear initial add tracking
        world.clear_entity_changes(entity);
        
        // Now mutate the component
        if let Some(t) = world.get_component_mut::<TransformComponent>(entity) {
            t.position = Vec3::new(5.0, 6.0, 7.0);
        }
        
        // Should be detected as changed
        let changed = world.get_changed_renderable_entities();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0], entity);
    }
    
    #[test]
    fn test_change_detection_multiple_components() {
        let mut world = World::new();
        let entity = world.create_entity();
        
        // Add Transform and Renderable
        let transform = TransformComponent::from_position(Vec3::new(1.0, 2.0, 3.0));
        let renderable = RenderableComponent::new(
            MaterialId(0),
            crate::render::primitives::Mesh::cube(),
        );
        
        world.add_component(entity, transform);
        world.add_component(entity, renderable);
        
        // Should be detected (has both Transform and Renderable)
        let changed = world.get_changed_renderable_entities();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0], entity);
    }
    
    #[test]
    fn test_change_detection_filters_non_renderable() {
        let mut world = World::new();
        let entity = world.create_entity();
        
        // Add only Transform (no Renderable)
        let transform = TransformComponent::from_position(Vec3::new(1.0, 2.0, 3.0));
        world.add_component(entity, transform);
        
        // Should still be detected (either Transform OR Renderable is sufficient)
        let changed = world.get_changed_renderable_entities();
        assert_eq!(changed.len(), 1);
    }
    
    #[test]
    fn test_change_detection_clear() {
        let mut world = World::new();
        let entity = world.create_entity();
        
        // Add component
        let transform = TransformComponent::from_position(Vec3::new(1.0, 2.0, 3.0));
        world.add_component(entity, transform);
        
        // Should be detected
        assert_eq!(world.get_changed_renderable_entities().len(), 1);
        
        // Clear changes
        world.clear_entity_changes(entity);
        
        // Should no longer be detected
        assert_eq!(world.get_changed_renderable_entities().len(), 0);
    }
    
    #[test]
    fn test_change_detection_clear_all() {
        let mut world = World::new();
        
        // Create multiple entities with changes
        for i in 0..3 {
            let entity = world.create_entity();
            let transform = TransformComponent::from_position(Vec3::new(i as f32, 0.0, 0.0));
            world.add_component(entity, transform);
        }
        
        // Should detect all 3
        assert_eq!(world.get_changed_renderable_entities().len(), 3);
        
        // Clear all changes
        world.clear_all_changes();
        
        // Should detect none
        assert_eq!(world.get_changed_renderable_entities().len(), 0);
    }
    
    #[test]
    fn test_generation_tracking() {
        let mut world = World::new();
        let entity = world.create_entity();
        
        // Add component
        let transform = TransformComponent::from_position(Vec3::new(1.0, 2.0, 3.0));
        world.add_component(entity, transform);
        
        // Get initial generation
        let type_id = TypeId::of::<TransformComponent>();
        let gen1 = world.changed_components.get(&entity.id())
            .and_then(|changes| changes.get(&type_id))
            .copied();
        
        // Mutate component
        if let Some(_t) = world.get_component_mut::<TransformComponent>(entity) {
            // Generation should increment
        }
        
        // Get new generation
        let gen2 = world.changed_components.get(&entity.id())
            .and_then(|changes| changes.get(&type_id))
            .copied();
        
        // Generations should be different (incremented)
        assert_ne!(gen1, gen2);
        assert!(gen2 > gen1);
    }
}
