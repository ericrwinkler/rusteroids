#version 450

// Camera UBO - Set 0, Binding 0 (unchanged from current shader)
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view_matrix;
    mat4 projection_matrix;
    mat4 view_projection_matrix;
    vec4 camera_position;
    vec4 camera_direction;
    vec2 viewport_size;
    vec2 near_far;
} camera;

// TEMPORARY: Use current SimpleLightingUBO for Step 1.2 validation
// This matches the existing UBO structure exactly
layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    vec4 directional_light_direction;
    vec4 directional_light_color;
    vec4 _padding;
} lighting;

// Push constants - unchanged from current shader
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
layout(location = 2) out vec3 fragWorldPos;

void main() {
    // IDENTICAL vertex processing from current vert_ubo.vert
    
    // Transform position to clip space
    vec4 worldPos = pushConstants.model_matrix * vec4(inPosition, 1.0);
    gl_Position = camera.view_projection_matrix * worldPos;
    
    // Transform normal to world space using normal matrix
    fragNormal = pushConstants.normal_matrix * inNormal;
    
    // Pass through texture coordinates
    fragTexCoord = inTexCoord;
    
    // Pass world position to fragment shader for point light distance calculations
    fragWorldPos = worldPos.xyz;
}
