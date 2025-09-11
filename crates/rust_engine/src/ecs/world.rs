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
}

impl<T: Component> ComponentStorage<T> {
    fn new() -> Self {
        Self {
            components: Vec::new(),
            entity_to_index: HashMap::new(),
            index_to_entity: Vec::new(),
            free_indices: Vec::new(),
        }
    }
    
    fn add(&mut self, entity_id: u32, component: T) {
        let index = if let Some(free_idx) = self.free_indices.pop() {
            // Reuse free slot
            self.components[free_idx] = component;
            self.index_to_entity[free_idx] = entity_id;
            free_idx
        } else {
            // Append new component
            let idx = self.components.len();
            self.components.push(component);
            self.index_to_entity.push(entity_id);
            idx
        };
        
        self.entity_to_index.insert(entity_id, index);
    }
    
    fn get(&self, entity_id: u32) -> Option<&T> {
        self.entity_to_index.get(&entity_id)
            .and_then(|&index| self.components.get(index))
    }
    
    fn get_mut(&mut self, entity_id: u32) -> Option<&mut T> {
        self.entity_to_index.get(&entity_id)
            .and_then(|&index| self.components.get_mut(index))
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
}

impl World {
    /// Create a new world
    pub fn new() -> Self {
        Self {
            next_entity_id: 0,
            entities: Vec::new(),
            alive_entities: HashMap::new(),
            component_storages: HashMap::new(),
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
    }
    
    /// Get a component from an entity
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.get_storage_ref::<T>()?.get(entity.id())
    }
    
    /// Get a mutable component from an entity
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let entity_id = entity.id();
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
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
