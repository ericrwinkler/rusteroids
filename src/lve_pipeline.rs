use nalgebra::Vector;
use std::io::{self, Read};

pub struct Lve_pipeline {
    vert_filepath: String,
    frag_filepath: String,
}

impl Lve_pipeline {
    pub fn new(vert_filepath: &str, frag_filepath: &str) -> Self {
        Lve_pipeline {
            vert_filepath: vert_filepath.to_string(),
            frag_filepath: frag_filepath.to_string(),
        }
    }

    fn readFile(filepath: &str) -> io::Result<Vec<u8>> {
        use std::fs::File;
        use std::io::{self, Read};

        let mut file = File::open(filepath)?;
        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    fn createGraphicsPipeline(&self) -> ash::vk::Pipeline {
        // This function will create the graphics pipeline
        // For now, we will just return a dummy pipeline
        // In the future, this will be replaced with actual Vulkan pipeline creation code

        let vert_shader_code =
            Self::readFile(&self.vert_filepath).expect("Failed to read vertex shader file");
        let frag_shader_code =
            Self::readFile(&self.frag_filepath).expect("Failed to read fragment shader file");

        // Return a dummy pipeline (this should be replaced with actual Vulkan pipeline creation)
        ash::vk::Pipeline::null()
    }
}
