//! Configuration system

pub use serde::{Serialize, Deserialize};

/// Configuration trait
pub trait Config: Serialize + for<'de> Deserialize<'de> + Default {
    /// Load configuration from file
    fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)
            .map_err(ConfigError::Io)?;
        
        // Try different formats
        if path.ends_with(".toml") {
            toml::from_str(&contents).map_err(|e| ConfigError::Parse(e.to_string()))
        } else if path.ends_with(".ron") {
            ron::from_str(&contents).map_err(|e| ConfigError::Parse(e.to_string()))
        } else {
            Err(ConfigError::UnsupportedFormat(path.to_string()))
        }
    }
    
    /// Save configuration to file
    fn save_to_file(&self, path: &str) -> Result<(), ConfigError> {
        let contents = if path.ends_with(".toml") {
            toml::to_string_pretty(self).map_err(|e| ConfigError::Serialize(e.to_string()))?
        } else if path.ends_with(".ron") {
            ron::ser::to_string_pretty(self, Default::default())
                .map_err(|e| ConfigError::Serialize(e.to_string()))?
        } else {
            return Err(ConfigError::UnsupportedFormat(path.to_string()));
        };
        
        std::fs::write(path, contents).map_err(ConfigError::Io)
    }
}

/// Configuration errors
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialize(String),
    
    /// Unsupported format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}
