#version 450

// Vertex attributes
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 texCoord;

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
    vec4 directional_light_direction;
    vec4 directional_light_color;
    vec4 _padding;
} lighting;

// Push constants - only model matrix and material ID
layout(push_constant) uniform PushConstants {
    mat4 model_matrix;
    mat3 normal_matrix;
    uint material_id;
    uint _padding1;
    uint _padding2;
    uint _padding3;
} pushConstants;

// Output to fragment shader
layout(location = 0) out vec3 fragPosition;
layout(location = 1) out vec3 fragNormal;
layout(location = 2) out vec2 fragTexCoord;
layout(location = 3) out vec3 fragCameraPosition;

void main() {
    // Transform vertex position to world space
    vec4 worldPosition = pushConstants.model_matrix * vec4(position, 1.0);
    
    // Transform to clip space
    gl_Position = camera.view_projection_matrix * worldPosition;
    
    // Pass world position to fragment shader
    fragPosition = worldPosition.xyz;
    
    // Transform normal to world space using normal matrix
    fragNormal = pushConstants.normal_matrix * normal;
    
    // Pass texture coordinates unchanged
    fragTexCoord = texCoord;
    
    // Pass camera position for lighting calculations
    fragCameraPosition = camera.camera_position.xyz;
}
