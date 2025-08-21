#version 450

// Camera UBO - Set 0, Binding 0
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view_matrix;
    mat4 projection_matrix;
    mat4 view_projection_matrix;
    vec4 camera_position;
    vec4 camera_direction;
    vec2 viewport_size;
    vec2 near_far;
} camera;

// Lighting UBO - Set 0, Binding 1  
layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    // For now, just support one directional light
    vec4 directional_light_direction;
    vec4 directional_light_color;
} lighting;

// Push constants - only model matrix and material data
layout(push_constant) uniform PushConstants {
    mat4 model_matrix;
    mat3 normal_matrix;
    vec4 material_color;
} pushConstants;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inTexCoord;

layout(location = 0) out vec3 fragNormal;
layout(location = 1) out vec2 fragTexCoord;

void main() {
    // Use pre-computed view-projection matrix from UBO and model matrix from push constants
    gl_Position = camera.view_projection_matrix * pushConstants.model_matrix * vec4(inPosition, 1.0);
    
    // Transform normal using the normal matrix from push constants
    fragNormal = normalize(pushConstants.normal_matrix * inNormal);
    
    fragTexCoord = inTexCoord;
}
