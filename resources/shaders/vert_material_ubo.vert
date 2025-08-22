#version 450

// Set 0: Per-frame data
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view_matrix;
    mat4 projection_matrix;
    mat4 view_projection_matrix;
    vec4 camera_position;
    vec4 camera_direction;
    vec2 viewport_size;
    vec2 near_far;
} camera;

layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    vec4 directional_light_direction;
    vec4 directional_light_color;
    vec4 _padding;
} lighting;

// Set 1: Per-material data (not used in vertex shader but declared for consistency)
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 base_color;
    vec4 metallic_roughness_ao_normal;
    vec4 emission;
    uvec4 texture_flags;
    vec4 additional_params;
    vec4 _padding;
} material;

// Push constants - minimal data (no material color since it's in UBO now)
layout(push_constant) uniform PushConstants {
    mat4 model_matrix;
    mat3 normal_matrix;
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
