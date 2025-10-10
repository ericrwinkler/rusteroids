//! System Scheduling and Dependency Management
//! 
//! Provides deterministic execution order and parallel system execution
//! while preventing race conditions and deadlocks.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;
use crossbeam::channel::{bounded, Receiver, Sender};

/// Unique identifier for systems
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SystemId(pub u64);

/// Component type identifier for conflict detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentType(pub u64);

/// System execution phases with explicit ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemPhase {
    /// Input processing, entity lifecycle management
    PreUpdate = 0,
    /// Game logic, physics simulation
    Update = 1,
    /// Animation, transform updates
    PostUpdate = 2,
    /// Render command generation (read-only)
    Render = 3,
    /// GPU command submission
    Present = 4,
}

/// System trait with dependency and conflict declaration
pub trait System: Send + Sync {
    /// Systems that must execute before this one
    fn dependencies(&self) -> &[SystemId];
    
    /// Component types this system will modify (for conflict detection)
    fn write_components(&self) -> &[ComponentType];
    
    /// Component types this system will read (for optimization)
    fn read_components(&self) -> &[ComponentType];
    
    /// Which phase this system belongs to
    fn phase(&self) -> SystemPhase;
    
    /// Execute the system
    fn execute(&mut self, world: &crate::ecs::World, delta_time: f32);
    
    /// System identifier
    fn id(&self) -> SystemId;
}

/// Dependency graph for system scheduling
pub struct DependencyGraph {
    nodes: HashMap<SystemId, SystemNode>,
    phases: HashMap<SystemPhase, Vec<SystemId>>,
}

#[derive(Debug)]
struct SystemNode {
    id: SystemId,
    dependencies: HashSet<SystemId>,
    dependents: HashSet<SystemId>,
    phase: SystemPhase,
    write_components: HashSet<ComponentType>,
    read_components: HashSet<ComponentType>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            phases: HashMap::new(),
        }
    }

    /// Add a system to the dependency graph
    pub fn add_system(&mut self, system: &dyn System) {
        let id = system.id();
        let phase = system.phase();
        
        let node = SystemNode {
            id,
            dependencies: system.dependencies().iter().copied().collect(),
            dependents: HashSet::new(),
            phase,
            write_components: system.write_components().iter().copied().collect(),
            read_components: system.read_components().iter().copied().collect(),
        };

        // Add to phase group
        self.phases.entry(phase).or_insert_with(Vec::new).push(id);
        
        // Update dependent relationships
        for &dep_id in system.dependencies() {
            if let Some(dep_node) = self.nodes.get_mut(&dep_id) {
                dep_node.dependents.insert(id);
            }
        }

        self.nodes.insert(id, node);
    }

    /// Generate execution plan with parallel batches
    pub fn generate_execution_plan(&self) -> ExecutionPlan {
        let mut plan = ExecutionPlan::new();
        
        // Process each phase in order
        let mut sorted_phases: Vec<_> = self.phases.keys().copied().collect();
        sorted_phases.sort();
        
        for phase in sorted_phases {
            if let Some(systems) = self.phases.get(&phase) {
                let phase_plan = self.generate_phase_plan(systems);
                plan.add_phase(phase, phase_plan);
            }
        }
        
        plan
    }

    /// Generate execution plan for a single phase
    fn generate_phase_plan(&self, systems: &[SystemId]) -> PhasePlan {
        let mut remaining: HashSet<SystemId> = systems.iter().copied().collect();
        let mut batches = Vec::new();
        
        while !remaining.is_empty() {
            let mut current_batch = Vec::new();
            let mut conflicts = HashSet::new();
            
            // Find systems that can run in parallel
            for &system_id in &remaining {
                if let Some(node) = self.nodes.get(&system_id) {
                    // Check if all dependencies are satisfied
                    let deps_satisfied = node.dependencies.iter()
                        .all(|dep| !remaining.contains(dep));
                    
                    // Check for component conflicts
                    let has_conflicts = node.write_components.iter()
                        .any(|comp| conflicts.contains(comp)) ||
                        node.read_components.iter()
                        .any(|comp| conflicts.contains(comp) && 
                             self.component_has_writers(&remaining, *comp));
                    
                    if deps_satisfied && !has_conflicts {
                        current_batch.push(system_id);
                        // Mark components as in use
                        conflicts.extend(&node.write_components);
                        conflicts.extend(&node.read_components);
                    }
                }
            }
            
            // Remove scheduled systems
            for &system_id in &current_batch {
                remaining.remove(&system_id);
            }
            
            if current_batch.is_empty() {
                panic!("Circular dependency detected or invalid system configuration");
            }
            
            batches.push(current_batch);
        }
        
        PhasePlan { batches }
    }

    /// Check if any remaining systems write to a component
    fn component_has_writers(&self, remaining: &HashSet<SystemId>, component: ComponentType) -> bool {
        remaining.iter().any(|&id| {
            self.nodes.get(&id)
                .map(|node| node.write_components.contains(&component))
                .unwrap_or(false)
        })
    }
}

/// Complete execution plan across all phases
pub struct ExecutionPlan {
    phases: Vec<(SystemPhase, PhasePlan)>,
}

/// Execution plan for a single phase
pub struct PhasePlan {
    batches: Vec<Vec<SystemId>>,
}

impl ExecutionPlan {
    fn new() -> Self {
        Self { phases: Vec::new() }
    }

    fn add_phase(&mut self, phase: SystemPhase, plan: PhasePlan) {
        self.phases.push((phase, plan));
    }

    /// Get phases in execution order
    pub fn phases(&self) -> &[(SystemPhase, PhasePlan)] {
        &self.phases
    }
}

impl PhasePlan {
    /// Get batches that can execute in parallel
    pub fn batches(&self) -> &[Vec<SystemId>] {
        &self.batches
    }
}

/// System scheduler with thread pool
pub struct SystemScheduler {
    systems: HashMap<SystemId, Box<dyn System>>,
    execution_plan: ExecutionPlan,
    thread_pool: ThreadPool,
}

impl SystemScheduler {
    pub fn new(thread_count: usize) -> Self {
        Self {
            systems: HashMap::new(),
            execution_plan: ExecutionPlan::new(),
            thread_pool: ThreadPool::new(thread_count),
        }
    }

    /// Add a system to the scheduler
    pub fn add_system(&mut self, system: Box<dyn System>) {
        let id = system.id();
        self.systems.insert(id, system);
    }

    /// Build execution plan from current systems
    pub fn build_plan(&mut self) {
        let mut graph = DependencyGraph::new();
        
        for system in self.systems.values() {
            graph.add_system(system.as_ref());
        }
        
        self.execution_plan = graph.generate_execution_plan();
    }

    /// Execute all systems for one frame
    pub fn execute_frame(&mut self, world: &crate::ecs::World, delta_time: f32) {
        for (phase, phase_plan) in self.execution_plan.phases() {
            for batch in phase_plan.batches() {
                if batch.len() == 1 {
                    // Single system - execute directly
                    let system_id = batch[0];
                    if let Some(system) = self.systems.get_mut(&system_id) {
                        system.execute(world, delta_time);
                    }
                } else {
                    // Multiple systems - execute in parallel
                    self.execute_parallel_batch(batch, world, delta_time);
                }
            }
        }
    }

    fn execute_parallel_batch(&mut self, batch: &[SystemId], world: &crate::ecs::World, delta_time: f32) {
        // This is a simplified version - real implementation would need
        // to safely share world access across threads
        for &system_id in batch {
            if let Some(system) = self.systems.get_mut(&system_id) {
                system.execute(world, delta_time);
            }
        }
    }
}

/// Simple thread pool for parallel execution
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Sender<Job>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let (sender, receiver) = bounded(100);
        let receiver = Arc::new(Mutex::new(receiver));
        
        let mut workers = Vec::with_capacity(size);
        
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }
        
        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F) 
    where 
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let receiver = receiver.lock().unwrap();
            match receiver.recv() {
                Ok(job) => {
                    drop(receiver);
                    job();
                }
                Err(_) => break,
            }
        });

        Worker { id, thread }
    }
}
