#version 450

// Per-frame uniform buffer (set 0, binding 0)
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view_matrix;
    mat4 projection_matrix;
    mat4 view_projection_matrix;
    vec4 camera_position;
    vec4 camera_direction;
    vec2 viewport_size;
    vec2 near_far;
} camera;

// Per-frame lighting uniform buffer (set 0, binding 1)
layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    // Directional lights (up to 4)
    vec4 directional_lights_direction[4];
    vec4 directional_lights_color[4];
    // Point lights (up to 8)
    vec4 point_lights_position[8];
    vec4 point_lights_color[8]; 
    vec4 point_lights_attenuation[8];
    // Spot lights (up to 4)
    vec4 spot_lights_position[4];
    vec4 spot_lights_direction[4];
    vec4 spot_lights_color[4];
    vec4 spot_lights_params[4];
    // Light counts
    uint num_dir_lights;
    uint num_point_lights;
    uint num_spot_lights;
    uint _padding;
} lighting;

// Per-material uniform buffer (set 1, binding 0)
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 base_color;
    vec4 metallic_roughness; // metallic, roughness, ao, _padding
    vec4 emission;
    float normal_scale;
    uint texture_flags;
    uint _padding1;
    uint _padding2;
} material;

// Push constants (minimal per-draw data)
layout(push_constant) uniform PushConstants {
    mat4 model_matrix;
    mat3 normal_matrix;
    uint material_id;
    uint _padding1;
    uint _padding2;  
    uint _padding3;
} pushConstants;

// Vertex input
layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inTexCoord;

// Fragment shader inputs
layout(location = 0) out vec3 fragWorldPos;
layout(location = 1) out vec3 fragNormal;
layout(location = 2) out vec2 fragTexCoord;
layout(location = 3) out vec3 fragViewPos;

void main() {
    // Transform vertex to world space
    vec4 worldPos = pushConstants.model_matrix * vec4(inPosition, 1.0);
    fragWorldPos = worldPos.xyz;
    
    // Transform to clip space using pre-computed view-projection matrix
    gl_Position = camera.view_projection_matrix * worldPos;
    
    // Transform normal to world space using the normal matrix
    fragNormal = normalize(pushConstants.normal_matrix * inNormal);
    
    // Transform vertex to view space for view-dependent effects
    vec4 viewPos = camera.view_matrix * worldPos;
    fragViewPos = viewPos.xyz;
    
    // Pass through texture coordinates
    fragTexCoord = inTexCoord;
}
