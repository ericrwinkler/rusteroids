//! Material creation and loading subsystem
//!
//! Unified material factory supporting both file-based (MTL) and procedural (builder) workflows.

pub mod mtl_parser;
pub mod material_loader;
pub mod material_builder;
pub mod material_cache;
pub mod material_factory;

pub use mtl_parser::{MtlParser, MtlData};
pub use material_loader::{MaterialLoader, LoadedMaterial, MaterialTexturePaths};
pub use material_builder::MaterialBuilder;
pub use material_cache::MaterialCache;
pub use material_factory::MaterialFactory;
